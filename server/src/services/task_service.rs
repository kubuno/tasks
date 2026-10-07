use chrono::{Duration, Utc};
use kubuno_db::dialect::Backend;
use kubuno_db::{params, DbPool, DbQueryBuilder, DbTx};
use uuid::Uuid;

use crate::{
    errors::{Result, TasksError},
    models::{
        label::Label,
        task::{CreateTaskDto, MoveTaskDto, Task, TaskWithMeta, TasksQuery, UpdateTaskDto},
    },
    services::board_service::BoardService,
    sync,
};

pub struct TaskService;

fn new_uid() -> String {
    format!("{}@kubuno.tasks", Uuid::new_v4())
}

/// The `INSERT ... ON CONFLICT DO NOTHING` for a task→label link, in the local
/// spelling. Kept in one place because it is written from three call sites.
fn task_label_insert_ignore(backend: Backend) -> String {
    format!(
        "INSERT {ignore}INTO tasks.task_labels (task_id, label_id) VALUES ($1, $2){nothing}",
        ignore = backend.insert_ignore_prefix(),
        nothing = backend.on_conflict_do_nothing(&["task_id", "label_id"]),
    )
}

fn task_assignee_insert_ignore(backend: Backend) -> String {
    format!(
        "INSERT {ignore}INTO tasks.task_assignees (task_id, user_id) VALUES ($1, $2){nothing}",
        ignore = backend.insert_ignore_prefix(),
        nothing = backend.on_conflict_do_nothing(&["task_id", "user_id"]),
    )
}

/// The next `position` at the end of a board's column (or of its unclassified
/// tasks when `stack_id` is `None`). The branch avoids `IS NOT DISTINCT FROM`,
/// which MariaDB does not accept (`<=>`), by choosing a plain `= $2` or an
/// `IS NULL` predicate at build time.
async fn end_position(db: &DbPool, board_id: Uuid, stack_id: Option<Uuid>) -> Result<f64> {
    let max: Option<f64> = match stack_id {
        Some(sid) => {
            db.fetch_scalar::<Option<f64>>(
                "SELECT MAX(position) FROM tasks.tasks WHERE board_id = $1 AND stack_id = $2",
                params![board_id, sid],
            )
            .await?
        }
        None => {
            db.fetch_scalar::<Option<f64>>(
                "SELECT MAX(position) FROM tasks.tasks WHERE board_id = $1 AND stack_id IS NULL",
                params![board_id],
            )
            .await?
        }
    };
    Ok(max.map(|m| m + 1.0).unwrap_or(0.0))
}

impl TaskService {
    /// Board d'une tâche (pour les vérifications d'accès en cascade).
    pub async fn board_of_task(task_id: Uuid, db: &DbPool) -> Result<Uuid> {
        db.fetch_optional_scalar::<Uuid>(
            "SELECT board_id FROM tasks.tasks WHERE id = $1",
            params![task_id],
        )
        .await?
        .ok_or_else(|| TasksError::NotFound(format!("Task {task_id}")))
    }

    /// Accès à une tâche = accès au board, OU être assigné à la tâche.
    pub async fn assert_task_access(task_id: Uuid, user_id: Uuid, min: &str, db: &DbPool) -> Result<()> {
        use crate::services::board_service::perm_rank;
        let board_id = Self::board_of_task(task_id, db).await?;

        if let Some(level) = BoardService::access_level(board_id, user_id, db).await? {
            if perm_rank(&level) >= perm_rank(min) {
                return Ok(());
            }
        }
        // Repli : assigné de la tâche → write.
        let assigned: Option<Uuid> = db
            .fetch_optional_scalar(
                "SELECT user_id FROM tasks.task_assignees WHERE task_id = $1 AND user_id = $2",
                params![task_id, user_id],
            )
            .await?;
        if assigned.is_some() && perm_rank("write") >= perm_rank(min) {
            return Ok(());
        }
        Err(TasksError::Forbidden)
    }

    /// Liste de tâches : par board/stack, ou via une collection intelligente
    /// (today/upcoming/overdue/important/completed/all) scopée aux boards accessibles.
    pub async fn list(user_id: Uuid, q: &TasksQuery, db: &DbPool) -> Result<Vec<Task>> {
        if let Some(board_id) = q.board_id {
            BoardService::assert_access(board_id, user_id, "read", db).await?;
        }

        let mut qb = DbQueryBuilder::new(db.backend(), "SELECT t.* FROM tasks.tasks t WHERE ");

        // Scope d'accès : tâche dans un board accessible, OU tâche qui m'est assignée.
        qb.push("(t.board_id IN (SELECT b.id FROM tasks.boards b LEFT JOIN tasks.board_shares bs ON bs.board_id = b.id WHERE b.owner_id = ")
            .push_bind(user_id)
            .push(" OR bs.shared_with = ")
            .push_bind(user_id)
            .push(") OR EXISTS (SELECT 1 FROM tasks.task_assignees ta_s WHERE ta_s.task_id = t.id AND ta_s.user_id = ")
            .push_bind(user_id)
            .push("))");

        if let Some(board_id) = q.board_id {
            qb.push(" AND t.board_id = ").push_bind(board_id);
        }
        if let Some(stack_id) = q.stack_id {
            qb.push(" AND t.stack_id = ").push_bind(stack_id);
        }
        if let Some(ref status) = q.status {
            qb.push(" AND t.status = ").push_bind(status.clone());
        }
        if let Some(assignee) = q.assignee {
            qb.push(" AND EXISTS (SELECT 1 FROM tasks.task_assignees ta WHERE ta.task_id = t.id AND ta.user_id = ")
                .push_bind(assignee)
                .push(")");
        }
        if let Some(label_id) = q.label_id {
            qb.push(" AND EXISTS (SELECT 1 FROM tasks.task_labels tl WHERE tl.task_id = t.id AND tl.label_id = ")
                .push_bind(label_id)
                .push(")");
        }
        if let Some(ref search) = q.search {
            // Portable case-insensitive contains: LOWER(col) LIKE a pattern that
            // is already lower-cased in Rust — identical on the three engines,
            // and the placeholder stays at the tail so `push_bind` fits it. The
            // pattern is bound twice (title, description): the placeholder
            // rewriter forbids reusing a number, so each carries its own bind.
            let pat = format!("%{}%", search.to_lowercase());
            qb.push(" AND (LOWER(t.title) LIKE ")
                .push_bind(pat.clone())
                .push(" OR LOWER(t.description) LIKE ")
                .push_bind(pat)
                .push(")");
        }
        if let Some(due_before) = q.due_before {
            qb.push(" AND t.due_at <= ").push_bind(due_before);
        }
        if let Some(due_after) = q.due_after {
            qb.push(" AND t.due_at >= ").push_bind(due_after);
        }

        // Collections intelligentes. The date arithmetic PostgreSQL did with
        // `::date`, `NOW()` and `INTERVAL` is done in Rust and bound, so it is
        // engine-agnostic (and honours UTC, the only time zone Kubuno stores).
        let now = Utc::now();
        match q.collection.as_deref() {
            Some("today") => {
                let start = now.date_naive().and_hms_opt(0, 0, 0).unwrap_or_default().and_utc();
                let end = start + Duration::days(1);
                qb.push(" AND t.due_at >= ").push_bind(start)
                    .push(" AND t.due_at < ").push_bind(end)
                    .push(" AND t.status NOT IN ('done','cancelled')");
            }
            Some("upcoming") => {
                let horizon = now + Duration::days(7);
                qb.push(" AND t.due_at > ").push_bind(now)
                    .push(" AND t.due_at <= ").push_bind(horizon)
                    .push(" AND t.status NOT IN ('done','cancelled')");
            }
            Some("overdue") => {
                qb.push(" AND t.due_at < ").push_bind(now)
                    .push(" AND t.status NOT IN ('done','cancelled')");
            }
            Some("starred") => {
                qb.push(" AND t.starred AND t.status NOT IN ('done','cancelled')");
            }
            Some("important") => {
                qb.push(" AND t.priority >= 6 AND t.status NOT IN ('done','cancelled')");
            }
            Some("completed") => {
                qb.push(" AND t.status = 'done'");
            }
            Some("all") | None => {}
            Some(other) => {
                return Err(TasksError::Validation(format!("collection inconnue: {other}")));
            }
        }

        if !q.include_subtasks {
            qb.push(" AND t.parent_task_id IS NULL");
        }

        qb.push_order_by("t.position ASC, t.sort_order ASC, t.created_at ASC");

        let rows = qb.fetch_all_as::<Task>(db).await?;
        Ok(rows)
    }

    pub async fn get(id: Uuid, user_id: Uuid, db: &DbPool) -> Result<Task> {
        Self::assert_task_access(id, user_id, "read", db).await?;
        db.fetch_optional_as::<Task>("SELECT * FROM tasks.tasks WHERE id = $1", params![id])
            .await?
            .ok_or_else(|| TasksError::NotFound(format!("Task {id}")))
    }

    pub async fn get_with_meta(id: Uuid, user_id: Uuid, db: &DbPool) -> Result<TaskWithMeta> {
        let task = Self::get(id, user_id, db).await?;
        Self::enrich(task, db).await
    }

    async fn enrich(task: Task, db: &DbPool) -> Result<TaskWithMeta> {
        let labels = db
            .fetch_all_as::<Label>(
                r#"
                SELECT l.* FROM tasks.labels l
                JOIN tasks.task_labels tl ON tl.label_id = l.id
                WHERE tl.task_id = $1
                ORDER BY l.title
                "#,
                params![task.id],
            )
            .await?;

        let assignees: Vec<(Uuid,)> = db
            .fetch_all_as("SELECT user_id FROM tasks.task_assignees WHERE task_id = $1", params![task.id])
            .await?;
        let assignees: Vec<Uuid> = assignees.into_iter().map(|r| r.0).collect();

        let count = db.backend().count_bigint("*");
        let subtask_count: i64 = db
            .fetch_scalar(
                &format!("SELECT {count} FROM tasks.tasks WHERE parent_task_id = $1"),
                params![task.id],
            )
            .await?;
        let comment_count: i64 = db
            .fetch_scalar(
                &format!("SELECT {count} FROM tasks.comments WHERE task_id = $1"),
                params![task.id],
            )
            .await?;

        Ok(TaskWithMeta { task, labels, assignees, subtask_count, comment_count })
    }

    pub async fn create(
        user_id: Uuid,
        dto: CreateTaskDto,
        instance: &crate::config::InstanceConfig,
        db: &DbPool,
    ) -> Result<TaskWithMeta> {
        BoardService::assert_access(dto.board_id, user_id, "write", db).await?;

        if instance.max_tasks_per_board > 0 {
            let count = db.backend().count_bigint("*");
            let held: i64 = db
                .fetch_scalar(
                    &format!("SELECT {count} FROM tasks.tasks WHERE board_id = $1"),
                    params![dto.board_id],
                )
                .await?;
            if held >= instance.max_tasks_per_board {
                return Err(TasksError::Validation(format!(
                    "Nombre maximal de tâches atteint pour ce tableau ({})",
                    instance.max_tasks_per_board
                )));
            }
        }

        // Cohérence stack ↔ board.
        if let Some(stack_id) = dto.stack_id {
            let ok: Option<Uuid> = db
                .fetch_optional_scalar(
                    "SELECT id FROM tasks.stacks WHERE id = $1 AND board_id = $2",
                    params![stack_id, dto.board_id],
                )
                .await?;
            if ok.is_none() {
                return Err(TasksError::Validation("stack_id n'appartient pas au board".into()));
            }
        }

        let status = dto.status.unwrap_or_else(|| "open".to_string());
        if !["open", "in_progress", "done", "cancelled"].contains(&status.as_str()) {
            return Err(TasksError::Validation("status invalide".into()));
        }
        let priority = dto.priority.unwrap_or(0);
        let (percent, completed_at) = if status == "done" {
            (100i16, Some(Utc::now()))
        } else {
            (dto.percent_complete.unwrap_or(0), None)
        };
        let all_day = dto.all_day.unwrap_or(false);
        let reminders = dto.reminders.unwrap_or_else(|| serde_json::json!([]));
        let uid = new_uid();
        let starred = dto.starred.unwrap_or(false);
        let starred_at = if starred { Some(Utc::now()) } else { None };
        let task_id = dto.id.unwrap_or_else(kubuno_db::new_id);
        let etag = sync::new_tag();
        let empty_files: Vec<Uuid> = Vec::new();

        // Position en fin de colonne/board. `IS NOT DISTINCT FROM` is not
        // portable (MariaDB spells it `<=>`), so branch on whether a column was
        // given rather than emit a null-safe operator.
        let position = end_position(db, dto.board_id, dto.stack_id).await?;

        let mut tx = db.begin().await?;
        let seq = sync::next_task_seq(&mut tx).await?;
        tx.execute(
            r#"
            INSERT INTO tasks.tasks
                (id, board_id, stack_id, parent_task_id, owner_id, title, description,
                 status, priority, percent_complete, due_at, start_at, completed_at,
                 all_day, color, rrule, reminders, ical_uid, position, linked_event_id,
                 starred, starred_at, etag, change_seq, linked_file_ids)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25)
            "#,
            params![
                task_id, dto.board_id, dto.stack_id, dto.parent_task_id, user_id, dto.title,
                dto.description, status, priority, percent, dto.due_at, dto.start_at, completed_at,
                all_day, dto.color, dto.rrule, reminders, uid, position, dto.linked_event_id,
                starred, starred_at, etag, seq, empty_files
            ],
        )
        .await?;

        if let Some(ref ids) = dto.label_ids {
            Self::set_labels(&mut tx, task_id, dto.board_id, ids).await?;
        }
        if let Some(ref ids) = dto.assignee_ids {
            Self::set_assignees(&mut tx, task_id, ids).await?;
        }
        tx.commit().await?;

        let task = db
            .fetch_one_as::<Task>("SELECT * FROM tasks.tasks WHERE id = $1", params![task_id])
            .await?;

        if let Some(ref ids) = dto.assignee_ids {
            for a in ids {
                Self::ensure_share_for_assignee(dto.board_id, user_id, *a, db).await?;
            }
        }

        Self::schedule_reminders(&task, user_id, db).await?;
        Self::enrich(task, db).await
    }

    pub async fn update(id: Uuid, user_id: Uuid, dto: UpdateTaskDto, db: &DbPool) -> Result<TaskWithMeta> {
        Self::assert_task_access(id, user_id, "write", db).await?;
        let board_id = Self::board_of_task(id, db).await?;

        let cur = db
            .fetch_one_as::<Task>("SELECT * FROM tasks.tasks WHERE id = $1", params![id])
            .await?;

        let title       = dto.title.unwrap_or(cur.title);
        let description = dto.description.or(cur.description);
        let status      = dto.status.unwrap_or(cur.status.clone());
        if !["open", "in_progress", "done", "cancelled"].contains(&status.as_str()) {
            return Err(TasksError::Validation("status invalide".into()));
        }
        let priority    = dto.priority.unwrap_or(cur.priority);
        let all_day     = dto.all_day.unwrap_or(cur.all_day);
        let color       = if dto.clear_color { None } else { dto.color.or(cur.color) };
        let rrule       = dto.rrule.or(cur.rrule);
        let reminders   = dto.reminders.unwrap_or(cur.reminders);
        let due_at      = dto.due_at.or(cur.due_at);
        let start_at    = dto.start_at.or(cur.start_at);
        let stack_id    = dto.stack_id.or(cur.stack_id);
        let parent_task_id = dto.parent_task_id.or(cur.parent_task_id);

        let (percent, completed_at) = if status == "done" {
            (dto.percent_complete.unwrap_or(100), cur.completed_at.or(Some(Utc::now())))
        } else {
            (dto.percent_complete.unwrap_or(cur.percent_complete), None)
        };

        let starred = dto.starred.unwrap_or(cur.starred);
        let starred_at = match (cur.starred, starred) {
            (false, true) => Some(Utc::now()),
            (_, false)    => None,
            _             => cur.starred_at,
        };

        let linked_event_id = if dto.clear_linked_event {
            None
        } else {
            dto.linked_event_id.or(cur.linked_event_id)
        };
        let etag = sync::new_tag();

        let mut tx = db.begin().await?;
        let seq = sync::next_task_seq(&mut tx).await?;
        // Placeholders must ascend in text order (the portable rewriter refuses
        // `$2 ... $1`), so the SET list takes $1.. and the WHERE key comes last.
        tx.execute(
            r#"
            UPDATE tasks.tasks
            SET stack_id = $1, parent_task_id = $2, title = $3, description = $4,
                status = $5, priority = $6, percent_complete = $7, due_at = $8,
                start_at = $9, completed_at = $10, all_day = $11, rrule = $12,
                reminders = $13, linked_event_id = $14, color = $15,
                starred = $16, starred_at = $17,
                sequence = sequence + 1, etag = $18, change_seq = $19
            WHERE id = $20
            "#,
            params![
                stack_id, parent_task_id, title, description, status, priority, percent,
                due_at, start_at, completed_at, all_day, rrule, reminders, linked_event_id,
                color, starred, starred_at, etag, seq, id
            ],
        )
        .await?;

        if let Some(ref ids) = dto.label_ids {
            Self::set_labels(&mut tx, id, board_id, ids).await?;
        }
        if let Some(ref ids) = dto.assignee_ids {
            Self::set_assignees(&mut tx, id, ids).await?;
        }
        tx.commit().await?;

        let task = db
            .fetch_one_as::<Task>("SELECT * FROM tasks.tasks WHERE id = $1", params![id])
            .await?;

        if let Some(ref ids) = dto.assignee_ids {
            for a in ids {
                Self::ensure_share_for_assignee(board_id, user_id, *a, db).await?;
            }
        }

        Self::schedule_reminders(&task, user_id, db).await?;
        Self::enrich(task, db).await
    }

    /// Marque une tâche comme terminée (statut done + 100%).
    pub async fn complete(id: Uuid, user_id: Uuid, db: &DbPool) -> Result<Task> {
        Self::assert_task_access(id, user_id, "write", db).await?;
        let etag = sync::new_tag();
        let mut tx = db.begin().await?;
        let seq = sync::next_task_seq(&mut tx).await?;
        tx.execute(
            r#"
            UPDATE tasks.tasks
            SET status = 'done', percent_complete = 100, completed_at = $1,
                sequence = sequence + 1, etag = $2, change_seq = $3
            WHERE id = $4
            "#,
            params![Utc::now(), etag, seq, id],
        )
        .await?;
        tx.commit().await?;
        db.fetch_one_as::<Task>("SELECT * FROM tasks.tasks WHERE id = $1", params![id])
            .await
            .map_err(Into::into)
    }

    pub async fn move_task(id: Uuid, user_id: Uuid, dto: MoveTaskDto, db: &DbPool) -> Result<Task> {
        let board_id = Self::board_of_task(id, db).await?;
        BoardService::assert_access(board_id, user_id, "write", db).await?;

        if let Some(stack_id) = dto.stack_id {
            let ok: Option<Uuid> = db
                .fetch_optional_scalar(
                    "SELECT id FROM tasks.stacks WHERE id = $1 AND board_id = $2",
                    params![stack_id, board_id],
                )
                .await?;
            if ok.is_none() {
                return Err(TasksError::Validation("stack cible hors du board".into()));
            }
        }

        let sort_order = dto.sort_order.unwrap_or(0);
        let etag = sync::new_tag();
        let mut tx = db.begin().await?;
        let seq = sync::next_task_seq(&mut tx).await?;
        tx.execute(
            r#"
            UPDATE tasks.tasks
            SET stack_id = $1, position = $2, sort_order = $3, etag = $4, change_seq = $5
            WHERE id = $6
            "#,
            params![dto.stack_id, dto.position, sort_order, etag, seq, id],
        )
        .await?;
        tx.commit().await?;
        db.fetch_one_as::<Task>("SELECT * FROM tasks.tasks WHERE id = $1", params![id])
            .await
            .map_err(Into::into)
    }

    pub async fn delete(id: Uuid, user_id: Uuid, db: &DbPool) -> Result<()> {
        let board_id = Self::board_of_task(id, db).await?;
        BoardService::assert_access(board_id, user_id, "write", db).await?;

        // The task's owner is what tasks_delta filters tombstones on (not the
        // acting user, who may be a board admin deleting someone else's task).
        let owner_id: Option<Uuid> = db
            .fetch_optional_scalar("SELECT owner_id FROM tasks.tasks WHERE id = $1", params![id])
            .await?;
        let owner_id = match owner_id {
            Some(o) => o,
            None => return Ok(()),
        };

        let mut tx = db.begin().await?;
        let seq = sync::next_task_seq(&mut tx).await?;
        tx.execute("DELETE FROM tasks.tasks WHERE id = $1", params![id]).await?;
        kubuno_db::journal::record_tombstone(&mut tx, sync::TASK_TOMBSTONES, id, owner_id, seq).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Déplace une ou plusieurs tâches (et leurs sous-tâches) vers un autre board.
    /// Retourne les ids des tâches racines effectivement déplacées.
    pub async fn move_to_board(
        user_id: Uuid,
        dto: crate::models::task::MoveToBoardDto,
        db: &DbPool,
    ) -> Result<Vec<Uuid>> {
        BoardService::assert_access(dto.target_board_id, user_id, "write", db).await?;

        // Colonne cible : celle fournie (validée), sinon la 1ʳᵉ du board cible.
        let target_stack: Option<Uuid> = match dto.target_stack_id {
            Some(s) => {
                let ok: Option<Uuid> = db
                    .fetch_optional_scalar(
                        "SELECT id FROM tasks.stacks WHERE id = $1 AND board_id = $2",
                        params![s, dto.target_board_id],
                    )
                    .await?;
                if ok.is_none() {
                    return Err(TasksError::Validation("colonne cible hors du board".into()));
                }
                Some(s)
            }
            None => db
                .fetch_optional_scalar::<Uuid>(
                    "SELECT id FROM tasks.stacks WHERE board_id = $1 ORDER BY sort_order, created_at LIMIT 1",
                    params![dto.target_board_id],
                )
                .await?,
        };

        let mut moved = Vec::new();

        for task_id in &dto.task_ids {
            let src_board: Option<Uuid> = db
                .fetch_optional_scalar(
                    "SELECT board_id FROM tasks.tasks WHERE id = $1 AND parent_task_id IS NULL",
                    params![task_id],
                )
                .await?;
            let src_board = match src_board {
                Some(b) => b,
                None => continue,
            };
            if src_board == dto.target_board_id {
                continue;
            }
            if BoardService::assert_access(src_board, user_id, "write", db).await.is_err() {
                continue;
            }

            // Tâche + toutes ses descendantes (WITH RECURSIVE runs on the three engines).
            let ids: Vec<(Uuid,)> = db
                .fetch_all_as(
                    r#"
                    WITH RECURSIVE sub AS (
                        SELECT id FROM tasks.tasks WHERE id = $1
                        UNION ALL
                        SELECT t.id FROM tasks.tasks t JOIN sub ON t.parent_task_id = sub.id
                    )
                    SELECT id FROM sub
                    "#,
                    params![task_id],
                )
                .await?;
            let ids: Vec<Uuid> = ids.into_iter().map(|r| r.0).collect();

            // Position en fin de colonne cible (committed state; this loop
            // commits per root, so a later root sees an earlier one's move).
            let position = end_position(db, dto.target_board_id, target_stack).await?;

            let mut tx = db.begin().await?;

            // Détacher les labels (propres au board source) sur tout le sous-arbre.
            let start = 1usize;
            let in_list = db.backend().in_list(start, ids.len());
            tx.execute(
                &format!("DELETE FROM tasks.task_labels WHERE task_id IN ({in_list})"),
                ids.iter().map(|&i| i.into()).collect(),
            )
            .await?;

            // Each moved task is a versioned entity: it gets its OWN fresh
            // change_seq (a shared seq would let LIMIT pagination drop rows with
            // an equal cursor), so the subtree is updated row by row rather than
            // as one bulk statement.
            for sub_id in &ids {
                let seq = sync::next_task_seq(&mut tx).await?;
                let etag = sync::new_tag();
                if sub_id == task_id {
                    tx.execute(
                        "UPDATE tasks.tasks SET board_id = $1, stack_id = $2, position = $3, etag = $4, change_seq = $5 WHERE id = $6",
                        params![dto.target_board_id, target_stack, position, etag, seq, sub_id],
                    )
                    .await?;
                } else {
                    tx.execute(
                        "UPDATE tasks.tasks SET board_id = $1, stack_id = NULL, etag = $2, change_seq = $3 WHERE id = $4",
                        params![dto.target_board_id, etag, seq, sub_id],
                    )
                    .await?;
                }
            }

            tx.commit().await?;
            moved.push(*task_id);
        }

        Ok(moved)
    }

    pub async fn list_subtasks(parent_id: Uuid, user_id: Uuid, db: &DbPool) -> Result<Vec<Task>> {
        Self::assert_task_access(parent_id, user_id, "read", db).await?;
        let rows = db
            .fetch_all_as::<Task>(
                "SELECT * FROM tasks.tasks WHERE parent_task_id = $1 ORDER BY position, created_at",
                params![parent_id],
            )
            .await?;
        Ok(rows)
    }

    // ── Assignés ────────────────────────────────────────────────────────────────

    async fn ensure_share_for_assignee(
        board_id: Uuid,
        acting_user: Uuid,
        assignee: Uuid,
        db: &DbPool,
    ) -> Result<()> {
        let board: Option<(Uuid, bool)> = db
            .fetch_optional_as("SELECT owner_id, is_default FROM tasks.boards WHERE id = $1", params![board_id])
            .await?;
        let (owner_id, is_default) = match board {
            Some(b) => b,
            None => return Ok(()),
        };
        if is_default || assignee == acting_user || assignee == owner_id {
            return Ok(());
        }
        let clause = db.backend().on_conflict_do_nothing(&["board_id", "shared_with"]);
        let ignore = db.backend().insert_ignore_prefix();
        db.execute(
            &format!(
                "INSERT {ignore}INTO tasks.board_shares (id, board_id, shared_with, permission)
                 VALUES ($1, $2, $3, 'write'){clause}"
            ),
            params![kubuno_db::new_id(), board_id, assignee],
        )
        .await?;
        Ok(())
    }

    pub async fn add_assignee(task_id: Uuid, user_id: Uuid, assignee: Uuid, db: &DbPool) -> Result<()> {
        Self::assert_task_access(task_id, user_id, "write", db).await?;
        let board_id = Self::board_of_task(task_id, db).await?;

        let mut tx = db.begin().await?;
        let sql = task_assignee_insert_ignore(tx.backend());
        tx.execute(&sql, params![task_id, assignee]).await?;
        sync::touch_task(&mut tx, task_id).await?;
        tx.commit().await?;

        Self::ensure_share_for_assignee(board_id, user_id, assignee, db).await?;
        Ok(())
    }

    pub async fn remove_assignee(task_id: Uuid, user_id: Uuid, assignee: Uuid, db: &DbPool) -> Result<()> {
        Self::assert_task_access(task_id, user_id, "write", db).await?;
        let mut tx = db.begin().await?;
        tx.execute(
            "DELETE FROM tasks.task_assignees WHERE task_id = $1 AND user_id = $2",
            params![task_id, assignee],
        )
        .await?;
        sync::touch_task(&mut tx, task_id).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn list_assignees(task_id: Uuid, user_id: Uuid, db: &DbPool) -> Result<Vec<Uuid>> {
        Self::assert_task_access(task_id, user_id, "read", db).await?;
        let rows: Vec<(Uuid,)> = db
            .fetch_all_as("SELECT user_id FROM tasks.task_assignees WHERE task_id = $1", params![task_id])
            .await?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    // ── Labels sur une tâche ──────────────────────────────────────────────────────

    pub async fn add_label(task_id: Uuid, user_id: Uuid, label_id: Uuid, db: &DbPool) -> Result<()> {
        let board_id = Self::board_of_task(task_id, db).await?;
        BoardService::assert_access(board_id, user_id, "write", db).await?;
        let ok: Option<Uuid> = db
            .fetch_optional_scalar(
                "SELECT id FROM tasks.labels WHERE id = $1 AND board_id = $2",
                params![label_id, board_id],
            )
            .await?;
        if ok.is_none() {
            return Err(TasksError::Validation("label hors du board".into()));
        }
        let mut tx = db.begin().await?;
        let sql = task_label_insert_ignore(tx.backend());
        tx.execute(&sql, params![task_id, label_id]).await?;
        sync::touch_task(&mut tx, task_id).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn remove_label(task_id: Uuid, user_id: Uuid, label_id: Uuid, db: &DbPool) -> Result<()> {
        let board_id = Self::board_of_task(task_id, db).await?;
        BoardService::assert_access(board_id, user_id, "write", db).await?;
        let mut tx = db.begin().await?;
        tx.execute(
            "DELETE FROM tasks.task_labels WHERE task_id = $1 AND label_id = $2",
            params![task_id, label_id],
        )
        .await?;
        sync::touch_task(&mut tx, task_id).await?;
        tx.commit().await?;
        Ok(())
    }

    // ── Helpers tx ────────────────────────────────────────────────────────────────

    async fn set_labels(
        tx: &mut DbTx,
        task_id: Uuid,
        board_id: Uuid,
        label_ids: &[Uuid],
    ) -> Result<()> {
        tx.execute("DELETE FROM tasks.task_labels WHERE task_id = $1", params![task_id])
            .await?;
        let backend = tx.backend();
        let ignore = backend.insert_ignore_prefix();
        let nothing = backend.on_conflict_do_nothing(&["task_id", "label_id"]);
        for lid in label_ids {
            // Ignore les labels hors du board. `$2`/`$3` both bind `lid` — the
            // rewriter forbids reusing a placeholder number, so it is bound twice.
            tx.execute(
                &format!(
                    "INSERT {ignore}INTO tasks.task_labels (task_id, label_id)
                     SELECT $1, $2 WHERE EXISTS (
                         SELECT 1 FROM tasks.labels WHERE id = $3 AND board_id = $4
                     ){nothing}"
                ),
                params![task_id, lid, lid, board_id],
            )
            .await?;
        }
        sync::touch_task(tx, task_id).await?;
        Ok(())
    }

    async fn set_assignees(tx: &mut DbTx, task_id: Uuid, assignee_ids: &[Uuid]) -> Result<()> {
        tx.execute("DELETE FROM tasks.task_assignees WHERE task_id = $1", params![task_id])
            .await?;
        let sql = task_assignee_insert_ignore(tx.backend());
        for uid in assignee_ids {
            tx.execute(&sql, params![task_id, uid]).await?;
        }
        sync::touch_task(tx, task_id).await?;
        Ok(())
    }

    /// (Re)planifie les rappels d'une tâche en fonction de son échéance.
    async fn schedule_reminders(task: &Task, user_id: Uuid, db: &DbPool) -> Result<()> {
        db.execute(
            "DELETE FROM tasks.scheduled_reminders WHERE task_id = $1 AND sent = FALSE",
            params![task.id],
        )
        .await?;

        let due = match task.due_at {
            Some(d) => d,
            None => return Ok(()),
        };
        let reminders = match task.reminders.as_array() {
            Some(arr) => arr.clone(),
            None => return Ok(()),
        };
        let ignore = db.backend().insert_ignore_prefix();
        let nothing = db.backend().on_conflict_do_nothing(&["id"]);
        for reminder in reminders {
            let minutes_before = reminder.get("minutes_before").and_then(|v| v.as_i64()).unwrap_or(15);
            let channel = reminder.get("type").and_then(|v| v.as_str()).unwrap_or("push").to_string();
            let remind_at = due - Duration::minutes(minutes_before);
            if remind_at > Utc::now() {
                db.execute(
                    &format!(
                        "INSERT {ignore}INTO tasks.scheduled_reminders (id, task_id, user_id, remind_at, channel)
                         VALUES ($1, $2, $3, $4, $5){nothing}"
                    ),
                    params![kubuno_db::new_id(), task.id, user_id, remind_at, channel],
                )
                .await?;
            }
        }
        Ok(())
    }
}
