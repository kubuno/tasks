//! Sync deltas for the local-first pull (boards / tasks) — same contract as the
//! office sub-modules: owner-scoped changes past `cursor` (monotonic change_seq),
//! live rows + tombstones, ordered, paginated. `kind ∈ modified | deleted`.
//! Board changes carry stacks/labels/board_comments inline; task changes carry
//! label ids, assignee ids and comments inline.

use axum::{
    extract::{Query, State},
    Extension, Json,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    errors::Result,
    middleware::TasksUser,
    models::{board::Board, comment::{BoardComment, Comment}, label::Label, stack::Stack, task::Task},
    state::AppState,
};

#[derive(serde::Deserialize)]
pub struct DeltaQuery {
    #[serde(default)]
    cursor: i64,
    limit: Option<i64>,
}

async fn union_rows(
    state: &AppState,
    user: Uuid,
    live: &str,
    tomb: &str,
    cursor: i64,
    limit: i64,
) -> Result<Vec<(Uuid, i64, String)>> {
    let rows: Vec<(Uuid, i64, String)> = sqlx::query_as(&format!(
        r#"SELECT id, change_seq, 'live'::text AS src FROM {live}
               WHERE owner_id = $1 AND change_seq > $2
           UNION ALL
           SELECT id, change_seq, 'tomb'::text AS src FROM {tomb}
               WHERE owner_id = $1 AND change_seq > $2
           ORDER BY change_seq
           LIMIT $3"#
    ))
    .bind(user)
    .bind(cursor)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;
    Ok(rows)
}

/// GET /boards/delta
pub async fn boards_delta(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Query(q): Query<DeltaQuery>,
) -> Result<Json<Value>> {
    let limit = q.limit.unwrap_or(200).clamp(1, 500);
    let rows = union_rows(&state, user.id, "tasks.boards", "tasks.board_tombstones", q.cursor, limit).await?;
    let has_more = rows.len() as i64 == limit;
    let new_cursor = rows.last().map(|r| r.1).unwrap_or(q.cursor);
    let live_ids: Vec<Uuid> = rows.iter().filter(|r| r.2 == "live").map(|r| r.0).collect();

    let boards: Vec<Board> = if live_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as::<_, Board>("SELECT * FROM tasks.boards WHERE id = ANY($1)")
            .bind(&live_ids)
            .fetch_all(&state.db)
            .await?
    };
    let stacks: Vec<Stack> = if live_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as::<_, Stack>(
            "SELECT * FROM tasks.stacks WHERE board_id = ANY($1) ORDER BY sort_order, created_at",
        )
        .bind(&live_ids)
        .fetch_all(&state.db)
        .await?
    };
    let labels: Vec<Label> = if live_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as::<_, Label>("SELECT * FROM tasks.labels WHERE board_id = ANY($1) ORDER BY title")
            .bind(&live_ids)
            .fetch_all(&state.db)
            .await?
    };
    let bcomments: Vec<BoardComment> = if live_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as::<_, BoardComment>(
            "SELECT * FROM tasks.board_comments WHERE board_id = ANY($1) ORDER BY created_at",
        )
        .bind(&live_ids)
        .fetch_all(&state.db)
        .await?
    };

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
    let mut changes = Vec::with_capacity(rows.len());
    for (id, seq, src) in &rows {
        if src == "tomb" {
            changes.push(json!({ "uuid": id, "kind": "deleted", "change_seq": seq }));
        } else if let Some(b) = board_map.get(id) {
            changes.push(json!({
                "uuid": id,
                "kind": "modified",
                "change_seq": seq,
                "board": b,
                "stacks": stack_map.get(id).unwrap_or(&empty_s),
                "labels": label_map.get(id).unwrap_or(&empty_l),
                "board_comments": bc_map.get(id).unwrap_or(&empty_c),
            }));
        }
    }
    Ok(Json(json!({ "changes": changes, "cursor": new_cursor, "has_more": has_more })))
}

/// GET /tasks/delta
pub async fn tasks_delta(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Query(q): Query<DeltaQuery>,
) -> Result<Json<Value>> {
    let limit = q.limit.unwrap_or(200).clamp(1, 500);
    let rows = union_rows(&state, user.id, "tasks.tasks", "tasks.task_tombstones", q.cursor, limit).await?;
    let has_more = rows.len() as i64 == limit;
    let new_cursor = rows.last().map(|r| r.1).unwrap_or(q.cursor);
    let live_ids: Vec<Uuid> = rows.iter().filter(|r| r.2 == "live").map(|r| r.0).collect();

    let tasks: Vec<Task> = if live_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as::<_, Task>("SELECT * FROM tasks.tasks WHERE id = ANY($1)")
            .bind(&live_ids)
            .fetch_all(&state.db)
            .await?
    };
    let labels: Vec<(Uuid, Uuid)> = if live_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as("SELECT task_id, label_id FROM tasks.task_labels WHERE task_id = ANY($1)")
            .bind(&live_ids)
            .fetch_all(&state.db)
            .await?
    };
    let assignees: Vec<(Uuid, Uuid)> = if live_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as("SELECT task_id, user_id FROM tasks.task_assignees WHERE task_id = ANY($1)")
            .bind(&live_ids)
            .fetch_all(&state.db)
            .await?
    };
    let comments: Vec<Comment> = if live_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as::<_, Comment>(
            "SELECT * FROM tasks.comments WHERE task_id = ANY($1) ORDER BY created_at",
        )
        .bind(&live_ids)
        .fetch_all(&state.db)
        .await?
    };
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
    let mut changes = Vec::with_capacity(rows.len());
    for (id, seq, src) in &rows {
        if src == "tomb" {
            changes.push(json!({ "uuid": id, "kind": "deleted", "change_seq": seq }));
        } else if let Some(t) = task_map.get(id) {
            changes.push(json!({
                "uuid": id,
                "kind": "modified",
                "change_seq": seq,
                "task": t,
                "labels": label_map.get(id).unwrap_or(&empty_u),
                "assignees": asg_map.get(id).unwrap_or(&empty_u),
                "comments": c_map.get(id).unwrap_or(&empty_c),
            }));
        }
    }
    Ok(Json(json!({ "changes": changes, "cursor": new_cursor, "has_more": has_more })))
}
