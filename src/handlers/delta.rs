//! Sync deltas for the local-first pull (boards / tasks): owner-scoped changes
//! past `cursor` (monotonic change_seq), live rows + tombstones, ordered,
//! paginated. `kind ∈ modified | deleted`. Board changes carry stacks / labels /
//! board_comments inline; task changes carry label ids, assignee ids and
//! comments inline.
//!
//! The change feed comes from `kubuno_db::journal::changes_since` (the portable
//! `live UNION ALL tombstones` the module used to build by hand); the live rows
//! are then fetched by id with `DbQueryBuilder::push_in`, which renders the
//! `IN (...)` list — and `IN (NULL)` for an empty page — on every engine.

use axum::{
    extract::{Query, State},
    Extension, Json,
};
use kubuno_db::{journal::Change, DbQueryBuilder};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::Result,
    middleware::TasksUser,
    models::{board::Board, comment::{BoardComment, Comment}, label::Label, stack::Stack, task::Task},
    state::AppState,
    sync,
};

#[derive(serde::Deserialize)]
pub struct DeltaQuery {
    #[serde(default)]
    cursor: i64,
    limit: Option<i64>,
}

/// `SELECT * FROM <table> WHERE <key> IN (<ids>) [ORDER BY <order>]`, built so
/// the `IN` list (or `IN (NULL)` when empty) is spelled for the pool's engine.
async fn select_in<T: kubuno_db::FromAnyRow>(
    state: &AppState,
    select: &str,
    key: &str,
    ids: &[Uuid],
    order: &'static str,
) -> Result<Vec<T>> {
    let mut qb = DbQueryBuilder::new(state.db.backend(), select);
    qb.push(" WHERE ").push(key).push_in(ids.iter().copied());
    if !order.is_empty() {
        qb.push(order);
    }
    Ok(qb.fetch_all_as::<T>(&state.db).await?)
}

fn cursor_and_more(changes: &[Change], prev: i64, limit: i64) -> (i64, bool) {
    let has_more = changes.len() as i64 == limit;
    let new_cursor = changes.last().map(|c| c.change_seq).unwrap_or(prev);
    (new_cursor, has_more)
}

/// GET /boards/delta
pub async fn boards_delta(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Query(q): Query<DeltaQuery>,
) -> Result<Json<Value>> {
    let limit = q.limit.unwrap_or(200).clamp(1, 500);
    let changes = kubuno_db::journal::changes_since(
        &state.db, sync::BOARDS_TABLE, sync::BOARD_TOMBSTONES, user.id, q.cursor, limit,
    )
    .await?;
    let (new_cursor, has_more) = cursor_and_more(&changes, q.cursor, limit);
    let live_ids: Vec<Uuid> = changes.iter().filter(|c| !c.deleted).map(|c| c.id).collect();

    let boards: Vec<Board> =
        select_in(&state, "SELECT * FROM tasks.boards", "id", &live_ids, "").await?;
    let stacks: Vec<Stack> = select_in(
        &state, "SELECT * FROM tasks.stacks", "board_id", &live_ids, " ORDER BY sort_order, created_at",
    )
    .await?;
    let labels: Vec<Label> = select_in(
        &state, "SELECT * FROM tasks.labels", "board_id", &live_ids, " ORDER BY title",
    )
    .await?;
    let bcomments: Vec<BoardComment> = select_in(
        &state, "SELECT * FROM tasks.board_comments", "board_id", &live_ids, " ORDER BY created_at",
    )
    .await?;

    let mut stack_map: std::collections::HashMap<Uuid, Vec<&Stack>> = Default::default();
    for s in &stacks {
        stack_map.entry(s.board_id).or_default().push(s);
    }
    let mut label_map: std::collections::HashMap<Uuid, Vec<&Label>> = Default::default();
    for l in &labels {
        label_map.entry(l.board_id).or_default().push(l);
    }
    let mut bc_map: std::collections::HashMap<Uuid, Vec<&BoardComment>> = Default::default();
    for c in &bcomments {
        bc_map.entry(c.board_id).or_default().push(c);
    }
    let board_map: std::collections::HashMap<Uuid, &Board> = boards.iter().map(|b| (b.id, b)).collect();

    let empty_s: Vec<&Stack> = Vec::new();
    let empty_l: Vec<&Label> = Vec::new();
    let empty_c: Vec<&BoardComment> = Vec::new();
    let mut out = Vec::with_capacity(changes.len());
    for c in &changes {
        if c.deleted {
            out.push(json!({ "uuid": c.id, "kind": "deleted", "change_seq": c.change_seq }));
        } else if let Some(b) = board_map.get(&c.id) {
            out.push(json!({
                "uuid": c.id,
                "kind": "modified",
                "change_seq": c.change_seq,
                "board": b,
                "stacks": stack_map.get(&c.id).unwrap_or(&empty_s),
                "labels": label_map.get(&c.id).unwrap_or(&empty_l),
                "board_comments": bc_map.get(&c.id).unwrap_or(&empty_c),
            }));
        }
    }
    Ok(Json(json!({ "changes": out, "cursor": new_cursor, "has_more": has_more })))
}

/// GET /tasks/delta
pub async fn tasks_delta(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Query(q): Query<DeltaQuery>,
) -> Result<Json<Value>> {
    let limit = q.limit.unwrap_or(200).clamp(1, 500);
    let changes = kubuno_db::journal::changes_since(
        &state.db, sync::TASKS_TABLE, sync::TASK_TOMBSTONES, user.id, q.cursor, limit,
    )
    .await?;
    let (new_cursor, has_more) = cursor_and_more(&changes, q.cursor, limit);
    let live_ids: Vec<Uuid> = changes.iter().filter(|c| !c.deleted).map(|c| c.id).collect();

    let tasks: Vec<Task> =
        select_in(&state, "SELECT * FROM tasks.tasks", "id", &live_ids, "").await?;
    let labels: Vec<(Uuid, Uuid)> = select_in(
        &state, "SELECT task_id, label_id FROM tasks.task_labels", "task_id", &live_ids, "",
    )
    .await?;
    let assignees: Vec<(Uuid, Uuid)> = select_in(
        &state, "SELECT task_id, user_id FROM tasks.task_assignees", "task_id", &live_ids, "",
    )
    .await?;
    let comments: Vec<Comment> = select_in(
        &state, "SELECT * FROM tasks.comments", "task_id", &live_ids, " ORDER BY created_at",
    )
    .await?;

    let mut label_map: std::collections::HashMap<Uuid, Vec<Uuid>> = Default::default();
    for (t, l) in labels {
        label_map.entry(t).or_default().push(l);
    }
    let mut asg_map: std::collections::HashMap<Uuid, Vec<Uuid>> = Default::default();
    for (t, u) in assignees {
        asg_map.entry(t).or_default().push(u);
    }
    let mut c_map: std::collections::HashMap<Uuid, Vec<&Comment>> = Default::default();
    for c in &comments {
        c_map.entry(c.task_id).or_default().push(c);
    }
    let task_map: std::collections::HashMap<Uuid, &Task> = tasks.iter().map(|t| (t.id, t)).collect();

    let empty_u: Vec<Uuid> = Vec::new();
    let empty_c: Vec<&Comment> = Vec::new();
    let mut out = Vec::with_capacity(changes.len());
    for c in &changes {
        if c.deleted {
            out.push(json!({ "uuid": c.id, "kind": "deleted", "change_seq": c.change_seq }));
        } else if let Some(t) = task_map.get(&c.id) {
            out.push(json!({
                "uuid": c.id,
                "kind": "modified",
                "change_seq": c.change_seq,
                "task": t,
                "labels": label_map.get(&c.id).unwrap_or(&empty_u),
                "assignees": asg_map.get(&c.id).unwrap_or(&empty_u),
                "comments": c_map.get(&c.id).unwrap_or(&empty_c),
            }));
        }
    }
    Ok(Json(json!({ "changes": out, "cursor": new_cursor, "has_more": has_more })))
}
