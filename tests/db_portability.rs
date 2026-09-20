//! Runs the tasks module's own migrations and its own services against a real
//! server of **each** engine, from a single compiled binary — the proof that the
//! engine is a run-time choice, not a build-time one, and that the three
//! recently-ported primitives (delta sync, the dynamic QueryBuilder, and the
//! JSON array column) behave identically on all of them.
//!
//! * SQLite always runs (a temp file, no server).
//! * PostgreSQL runs when `KUBUNO_PG_TEST_URL` points at a throwaway database.
//! * MySQL/MariaDB runs when `KUBUNO_MYSQL_TEST_URL` does.
//!
//! ```sh
//! KUBUNO_PG_TEST_URL=postgres://u:p@127.0.0.1:5433/tasks \
//! KUBUNO_MYSQL_TEST_URL=mysql://u:p@127.0.0.1:3307/tasks \
//!   SQLX_OFFLINE=true cargo test --test db_portability
//! ```

use kubuno_tasks::config::InstanceConfig;
use kubuno_tasks::models::board::CreateBoardDto;
use kubuno_tasks::models::comment::CreateCommentDto;
use kubuno_tasks::models::stack::CreateStackDto;
use kubuno_tasks::models::task::{CreateTaskDto, Task, TasksQuery, UpdateTaskDto};
use kubuno_tasks::services::{
    board_service::BoardService, comment_service::CommentService, stack_service::StackService,
    task_service::TaskService,
};
use kubuno_tasks::{sync, SCHEMA};
use kubuno_db::params;
use uuid::Uuid;

fn base_settings(engine: &str) -> kubuno_db::DbSettings {
    kubuno_db::DbSettings {
        engine: engine.to_string(),
        url: None,
        host: None,
        port: None,
        user: None,
        password: None,
        database: None,
        path: None,
        max_connections: 4,
        min_connections: 0,
        connect_timeout: std::time::Duration::from_secs(10),
        run_migrations: true,
    }
}

/// Migrations run one at a time: the PostgreSQL and MySQL suites may share a server.
static EXCLUSIVE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn migrated_pool(settings: kubuno_db::DbSettings) -> (kubuno_db::DbPool, impl Sized) {
    let guard = EXCLUSIVE.lock().await;
    let pool = kubuno_db::connect(&settings, SCHEMA).await.expect("connect");

    kubuno_db::migrations!(
        "./migrations/postgres",
        "./migrations/mysql",
        "./migrations/sqlite",
    )
    .run(&pool, SCHEMA)
    .await
    .expect("migrations");

    kubuno_db::events::ensure_outbox(&pool, SCHEMA).await.expect("outbox");

    (pool, guard)
}

fn mk_task(board_id: Uuid, title: &str, status: &str) -> CreateTaskDto {
    CreateTaskDto {
        id: None,
        board_id,
        stack_id: None,
        parent_task_id: None,
        title: title.to_string(),
        description: None,
        status: Some(status.to_string()),
        priority: Some(0),
        starred: Some(false),
        percent_complete: None,
        due_at: None,
        start_at: None,
        all_day: None,
        color: None,
        rrule: None,
        reminders: None,
        label_ids: None,
        assignee_ids: None,
        linked_event_id: None,
    }
}

/// The greatest task-domain change_seq an owner can currently see (0 if none).
/// Reads through the very primitive the port introduced: `changes_since`.
async fn max_task_seq(pool: &kubuno_db::DbPool, owner: Uuid) -> i64 {
    let changes = kubuno_db::journal::changes_since(
        pool, sync::TASKS_TABLE, sync::TASK_TOMBSTONES, owner, 0, 10_000,
    )
    .await
    .expect("tasks delta");
    changes.iter().map(|c| c.change_seq).max().unwrap_or(0)
}

async fn max_board_seq(pool: &kubuno_db::DbPool, owner: Uuid) -> i64 {
    let changes = kubuno_db::journal::changes_since(
        pool, sync::BOARDS_TABLE, sync::BOARD_TOMBSTONES, owner, 0, 10_000,
    )
    .await
    .expect("boards delta");
    changes.iter().map(|c| c.change_seq).max().unwrap_or(0)
}

async fn full_suite(pool: &kubuno_db::DbPool) {
    let user = Uuid::new_v4();
    let instance = InstanceConfig::default();

    // ── create / list a board (also seeds three default kanban stacks) ──
    let board = BoardService::create(
        user,
        CreateBoardDto {
            id: None,
            initial_stack_ids: None,
            title: "Project".into(),
            description: None,
            color: None,
            board_type: Some("kanban".into()),
        },
        pool,
    )
    .await
    .expect("create board");
    assert_eq!(board.title, "Project");

    let stacks = StackService::list(board.id, user, pool).await.expect("stacks");
    assert_eq!(stacks.len(), 3, "kanban board seeds three stacks");

    // The board's creation gave it a change_seq; adding a child must move it.
    let board_seq_0 = max_board_seq(pool, user).await;
    assert!(board_seq_0 > 0, "the board carries a change_seq after creation");

    // ── create / list tasks + prove strict change_seq monotonicity ──
    let mut seqs: Vec<i64> = Vec::new();
    seqs.push(max_task_seq(pool, user).await); // 0, no tasks yet

    let a = TaskService::create(user, mk_task(board.id, "Alpha task", "open"), &instance, pool)
        .await
        .expect("create A");
    seqs.push(max_task_seq(pool, user).await);

    let b = TaskService::create(user, mk_task(board.id, "Beta task", "in_progress"), &instance, pool)
        .await
        .expect("create B");
    seqs.push(max_task_seq(pool, user).await);

    let list = TaskService::list(user, &TasksQuery::default(), pool).await.expect("list");
    assert_eq!(list.len(), 2, "both root tasks listed");

    // Dynamic search (the QueryBuilder path): case-insensitive title contains.
    let mut q = TasksQuery { search: Some("ALPHA".into()), ..Default::default() };
    let found = TaskService::list(user, &q, pool).await.expect("search");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, a.task.id, "search matched only Alpha");

    // Dynamic filter: by status.
    q = TasksQuery { status: Some("in_progress".into()), ..Default::default() };
    let by_status = TaskService::list(user, &q, pool).await.expect("status filter");
    assert_eq!(by_status.len(), 1);
    assert_eq!(by_status[0].id, b.task.id);

    // update A → its change_seq must advance past B's create.
    let upd = UpdateTaskDto {
        stack_id: None,
        parent_task_id: None,
        title: Some("Alpha task (edited)".into()),
        description: None,
        status: None,
        priority: Some(7),
        starred: Some(true),
        percent_complete: None,
        due_at: None,
        start_at: None,
        all_day: None,
        color: None,
        clear_color: false,
        rrule: None,
        reminders: None,
        label_ids: None,
        assignee_ids: None,
        linked_event_id: None,
        clear_linked_event: false,
    };
    let a_upd = TaskService::update(a.task.id, user, upd, pool).await.expect("update A");
    assert_eq!(a_upd.task.title, "Alpha task (edited)");
    assert!(a_upd.task.starred);
    seqs.push(max_task_seq(pool, user).await);

    // touch parent: a comment on A bumps A's task change_seq (child → parent).
    CommentService::create(a.task.id, user, CreateCommentDto { id: None, body: "note".into() }, pool)
        .await
        .expect("comment");
    seqs.push(max_task_seq(pool, user).await);

    // delete B → a task tombstone with a fresh (greater) change_seq.
    TaskService::delete(b.task.id, user, pool).await.expect("delete B");
    seqs.push(max_task_seq(pool, user).await);

    // EVERY step advanced the sequence: strict monotonicity of next_seq.
    for w in seqs.windows(2) {
        assert!(w[1] > w[0], "change_seq must strictly increase: {seqs:?}");
    }

    // The deletion surfaces as a tombstone (deleted = true) in the delta feed.
    let tombs = kubuno_db::journal::changes_since(
        pool, sync::TASKS_TABLE, sync::TASK_TOMBSTONES, user, 0, 10_000,
    )
    .await
    .expect("delta");
    assert!(
        tombs.iter().any(|c| c.id == b.task.id && c.deleted),
        "deleted task B must appear as a tombstone"
    );
    assert!(
        tombs.iter().any(|c| c.id == a.task.id && !c.deleted),
        "live task A must appear as a modified row"
    );

    // touch parent (board side): adding a stack bumps the board's change_seq.
    StackService::create(
        board.id,
        user,
        CreateStackDto { id: None, title: "Backlog".into(), sort_order: None },
        pool,
    )
    .await
    .expect("create stack");
    let board_seq_1 = max_board_seq(pool, user).await;
    assert!(board_seq_1 > board_seq_0, "a child write bumped the board's change_seq");

    // ── the JSON array column (linked_file_ids) round-trips and filters ──
    // The service surface does not expose it, so exercise the column directly:
    // write a Vec<Uuid> (DbValue::Json), read it back (#[sqlx(json)]), and filter
    // with the portable json_array_contains.
    let file_a = Uuid::new_v4();
    let file_b = Uuid::new_v4();
    pool.execute(
        "UPDATE tasks.tasks SET linked_file_ids = $1 WHERE id = $2",
        params![vec![file_a, file_b], a.task.id],
    )
    .await
    .expect("write array column");

    let reread = pool
        .fetch_one_as::<Task>("SELECT * FROM tasks.tasks WHERE id = $1", params![a.task.id])
        .await
        .expect("reread task");
    assert_eq!(reread.linked_file_ids, vec![file_a, file_b], "array column round-trips");

    let contains = pool.backend().json_array_contains("linked_file_ids", 1);
    let matched: Vec<Task> = pool
        .fetch_all_as(
            &format!("SELECT * FROM tasks.tasks WHERE {contains}"),
            params![file_a.to_string()],
        )
        .await
        .expect("json_array_contains");
    assert!(matched.iter().any(|t| t.id == a.task.id), "filter found the task by a linked file id");
    let none: Vec<Task> = pool
        .fetch_all_as(
            &format!("SELECT * FROM tasks.tasks WHERE {contains}"),
            params![Uuid::new_v4().to_string()],
        )
        .await
        .expect("json_array_contains miss");
    assert!(none.is_empty(), "an unrelated file id matches nothing");

    // ── delete the board: its tasks get tombstones, the board gets one too ──
    BoardService::delete(board.id, user, pool).await.expect("delete board");
    let board_changes = kubuno_db::journal::changes_since(
        pool, sync::BOARDS_TABLE, sync::BOARD_TOMBSTONES, user, 0, 10_000,
    )
    .await
    .expect("board delta");
    assert!(
        board_changes.iter().any(|c| c.id == board.id && c.deleted),
        "deleted board must appear as a tombstone"
    );
    let task_changes = kubuno_db::journal::changes_since(
        pool, sync::TASKS_TABLE, sync::TASK_TOMBSTONES, user, 0, 10_000,
    )
    .await
    .expect("task delta");
    assert!(
        task_changes.iter().any(|c| c.id == a.task.id && c.deleted),
        "the board's cascade must tombstone its surviving task A"
    );
}

#[tokio::test]
async fn sqlite_from_the_one_binary() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut s = base_settings("sqlite");
    s.path = Some(dir.path().to_string_lossy().into_owned());
    let (pool, _keep) = migrated_pool(s).await;
    full_suite(&pool).await;
}

#[tokio::test]
async fn postgres_from_the_one_binary() {
    let Ok(url) = std::env::var("KUBUNO_PG_TEST_URL") else {
        eprintln!("skipping: KUBUNO_PG_TEST_URL not set");
        return;
    };
    let mut s = base_settings("postgres");
    s.url = Some(url);
    let (pool, _keep) = migrated_pool(s).await;
    full_suite(&pool).await;
}

#[tokio::test]
async fn mysql_from_the_one_binary() {
    let Ok(url) = std::env::var("KUBUNO_MYSQL_TEST_URL") else {
        eprintln!("skipping: KUBUNO_MYSQL_TEST_URL not set");
        return;
    };
    let mut s = base_settings("mysql");
    s.url = Some(url);
    let (pool, _keep) = migrated_pool(s).await;
    full_suite(&pool).await;
}
