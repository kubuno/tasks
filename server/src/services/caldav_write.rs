//! CalDAV `PUT` of a task, scoped to the board of the token.
//!
//! A CalDAV token opens ONE board. `ical_uid` is unique across the whole
//! instance (`tasks.tasks.ical_uid UNIQUE`), so an upsert keyed on the UID alone
//! let a client holding any board's token overwrite the task of another board
//! (another user's) just by naming its UID. The write is therefore split: a UID
//! that is new is inserted into the token's board, a UID of the token's board is
//! updated there (the `UPDATE` itself filters on the board, so a task created
//! elsewhere in between is never touched), and a UID that belongs to another
//! board is a conflict.

use kubuno_db::{params, DbPool};
use uuid::Uuid;

use crate::{services::icalendar_service::ParsedVtodo, sync};

/// What a PUT did, or why it was refused.
#[derive(Debug, PartialEq, Eq)]
pub enum CaldavPut {
    Created(String),
    Updated(String),
    /// `If-None-Match: *` on a UID this board already holds.
    PreconditionFailed,
    /// The UID belongs to a task outside this board.
    Conflict,
}

/// What to do with a PUT, given the board currently holding the UID (if any).
#[derive(Debug, PartialEq, Eq)]
pub enum UidDecision {
    Create,
    Update,
    PreconditionFailed,
    Conflict,
}

pub fn decide(holder: Option<Uuid>, board_id: Uuid, if_none_match_star: bool) -> UidDecision {
    match holder {
        None => UidDecision::Create,
        Some(b) if b != board_id => UidDecision::Conflict,
        Some(_) if if_none_match_star => UidDecision::PreconditionFailed,
        Some(_) => UidDecision::Update,
    }
}

/// Writes `todo` under `ical_uid` into `board_id` (owned by `owner_id`).
pub async fn put_task(
    db: &DbPool,
    board_id: Uuid,
    owner_id: Uuid,
    ical_uid: &str,
    todo: &ParsedVtodo,
    if_none_match_star: bool,
) -> Result<CaldavPut, sqlx::Error> {
    let holder: Option<Uuid> = db
        .fetch_optional_scalar("SELECT board_id FROM tasks.tasks WHERE ical_uid = $1", params![ical_uid])
        .await?;
    let decision = decide(holder, board_id, if_none_match_star);
    let etag = sync::new_tag();
    match decision {
        UidDecision::Conflict => Ok(CaldavPut::Conflict),
        UidDecision::PreconditionFailed => Ok(CaldavPut::PreconditionFailed),
        UidDecision::Create => {
            let reminders = serde_json::json!([]);
            let empty_files: Vec<Uuid> = Vec::new();
            let mut tx = db.begin().await?;
            let seq = sync::next_task_seq(&mut tx).await?;
            let inserted = tx
                .execute(
                    "INSERT INTO tasks.tasks
                       (id, board_id, owner_id, title, description, status, priority, percent_complete,
                        due_at, start_at, completed_at, rrule, reminders, ical_uid, etag, change_seq, linked_file_ids)
                     VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17)",
                    params![
                        kubuno_db::new_id(), board_id, owner_id, todo.summary.clone(), todo.description.clone(),
                        todo.status.clone(), todo.priority, todo.percent_complete, todo.due_at, todo.start_at,
                        todo.completed_at, todo.rrule.clone(), reminders, ical_uid.to_string(), etag.clone(), seq,
                        empty_files
                    ],
                )
                .await;
            match inserted {
                Ok(_) => {
                    tx.commit().await?;
                    Ok(CaldavPut::Created(etag))
                }
                // The UID was taken in between (unique index): a conflict, not an overwrite.
                Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                    tx.rollback().await?;
                    Ok(CaldavPut::Conflict)
                }
                Err(e) => Err(e),
            }
        }
        UidDecision::Update => {
            let mut tx = db.begin().await?;
            let seq = sync::next_task_seq(&mut tx).await?;
            let rows = tx
                .execute(
                    "UPDATE tasks.tasks SET title = $1, description = $2, status = $3, priority = $4,
                       percent_complete = $5, due_at = $6, start_at = $7, completed_at = $8, rrule = $9,
                       sequence = sequence + 1, etag = $10, change_seq = $11
                     WHERE ical_uid = $12 AND board_id = $13",
                    params![
                        todo.summary.clone(), todo.description.clone(), todo.status.clone(), todo.priority,
                        todo.percent_complete, todo.due_at, todo.start_at, todo.completed_at, todo.rrule.clone(),
                        etag.clone(), seq, ical_uid.to_string(), board_id
                    ],
                )
                .await?;
            if rows == 0 {
                tx.rollback().await?;
                return Ok(CaldavPut::Conflict);
            }
            tx.commit().await?;
            Ok(CaldavPut::Updated(etag))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_uid_of_another_board_is_a_conflict() {
        let (mine, other) = (Uuid::new_v4(), Uuid::new_v4());
        assert_eq!(decide(None, mine, false), UidDecision::Create);
        assert_eq!(decide(None, mine, true), UidDecision::Create);
        assert_eq!(decide(Some(mine), mine, false), UidDecision::Update);
        assert_eq!(decide(Some(mine), mine, true), UidDecision::PreconditionFailed);
        assert_eq!(decide(Some(other), mine, false), UidDecision::Conflict);
        assert_eq!(decide(Some(other), mine, true), UidDecision::Conflict);
    }
}
