//! Delta-sync plumbing shared by the services and the delta handler.
//!
//! The local-first pull (boards / tasks) rests on a monotonic `change_seq` per
//! record and a tombstone per deletion. On PostgreSQL that used to be a
//! `SEQUENCE` plus `BEFORE UPDATE` / `AFTER DELETE` triggers; here it is the
//! portable [`kubuno_db::journal`] primitive, driven from Rust at every write
//! site. This module holds the literal table / domain names those calls take —
//! all `&'static str`, never request data — so the write sites read uniformly
//! and a rename happens in one place.
//!
//! Two entities are versioned: **boards** and **tasks**. Their children bump
//! the versioned parent instead of carrying their own sequence:
//!
//! * stacks / labels / board_comments  → bump their **board**
//! * comments / task_labels / task_assignees → bump their **task**
//!
//! (Attachments and board_shares are carried by neither delta feed, so they do
//! not bump anything — matching the trigger design they replace.)

use uuid::Uuid;

/// One shared counter table per schema; `next_seq` keys it by domain.
pub const CHANGE_COUNTER: &str = "tasks.change_counter";

pub const BOARDS_TABLE: &str = "tasks.boards";
pub const TASKS_TABLE: &str = "tasks.tasks";
pub const BOARD_TOMBSTONES: &str = "tasks.board_tombstones";
pub const TASK_TOMBSTONES: &str = "tasks.task_tombstones";

/// Logical counter domains (the row keys in `change_counter`).
pub const BOARD_DOMAIN: &str = "boards";
pub const TASK_DOMAIN: &str = "tasks";

/// A fresh opaque resource tag, replacing the non-portable `md5(random()::text)`
/// the migrations used. 32 lowercase hex chars, fitting the `VARCHAR(64)` etag
/// and ctag columns on every engine.
pub fn new_tag() -> String {
    Uuid::new_v4().simple().to_string()
}

/// The next monotonic sequence for the **boards** domain, taken inside `tx`.
pub async fn next_board_seq(tx: &mut kubuno_db::DbTx) -> Result<i64, sqlx::Error> {
    kubuno_db::journal::next_seq(tx, CHANGE_COUNTER, BOARD_DOMAIN).await
}

/// The next monotonic sequence for the **tasks** domain, taken inside `tx`.
pub async fn next_task_seq(tx: &mut kubuno_db::DbTx) -> Result<i64, sqlx::Error> {
    kubuno_db::journal::next_seq(tx, CHANGE_COUNTER, TASK_DOMAIN).await
}

/// Bumps a **board** to a fresh sequence — the portable replacement for the old
/// child-triggered `UPDATE ... SET change_seq = change_seq` no-op. Called after
/// any write to a stack, label or board comment.
pub async fn touch_board(tx: &mut kubuno_db::DbTx, board_id: Uuid) -> Result<(), sqlx::Error> {
    kubuno_db::journal::touch(tx, BOARDS_TABLE, CHANGE_COUNTER, BOARD_DOMAIN, "id", board_id)
        .await
        .map(|_| ())
}

/// Bumps a **task** to a fresh sequence. Called after any write to a comment,
/// a task label link or a task assignee.
pub async fn touch_task(tx: &mut kubuno_db::DbTx, task_id: Uuid) -> Result<(), sqlx::Error> {
    kubuno_db::journal::touch(tx, TASKS_TABLE, CHANGE_COUNTER, TASK_DOMAIN, "id", task_id)
        .await
        .map(|_| ())
}
