//! Retention of completed tasks.
//!
//! An administration that sets a retention is asking the instance to stop
//! keeping a done-list nobody reads — a data-minimisation decision, so it is the
//! server that applies it and not whichever client happens to be open. The knob
//! is `completed_task_retention_days`; left at `0` (the default) this worker does
//! nothing at all, and says nothing either.
//!
//! What is deliberately NOT purged: a task that is done but whose subtasks are
//! not. Deleting it would cascade over work still in progress, and "completed" is
//! a claim about the parent alone. A fully finished tree does go, subtasks
//! included — and, because the FK cascade fires no application code, this worker
//! writes a task tombstone for every row the cascade will remove (exactly what
//! the old AFTER DELETE trigger did), so the delta feed learns each deletion.

use chrono::{Duration, Utc};
use kubuno_db::params;
use uuid::Uuid;

use crate::{state::AppState, sync};

/// How often the cleaner wakes up.
const SWEEP_INTERVAL: std::time::Duration = std::time::Duration::from_secs(6 * 3600);

/// Root tasks considered per pass (each may drag a subtree with it).
const BATCH: i64 = 500;

pub struct RetentionService;

impl RetentionService {
    /// Runs the sweep forever. First pass 5 minutes after startup.
    pub async fn run_worker(state: AppState) {
        tokio::time::sleep(std::time::Duration::from_secs(300)).await;
        loop {
            Self::sweep(&state).await;
            tokio::time::sleep(SWEEP_INTERVAL).await;
        }
    }

    /// One pass. Reads the retention at the last moment so an admin edit takes
    /// effect on the next sweep, and stops as soon as a batch comes back short.
    pub async fn sweep(state: &AppState) {
        let days = state.instance().completed_task_retention_days;
        if days <= 0 {
            return;
        }
        let cutoff = Utc::now() - Duration::days(days);
        let db = &state.db;

        let mut total: u64 = 0;
        loop {
            let doomed: Vec<(Uuid,)> = match db
                .fetch_all_as(
                    r#"
                    SELECT t.id
                    FROM tasks.tasks t
                    WHERE t.status = 'done'
                      AND t.completed_at IS NOT NULL
                      AND t.completed_at < $1
                      AND NOT EXISTS (
                          SELECT 1 FROM tasks.tasks c
                          WHERE c.parent_task_id = t.id
                            AND c.status <> 'done'
                      )
                    LIMIT $2
                    "#,
                    params![cutoff, BATCH],
                )
                .await
            {
                Ok(rows) => rows,
                Err(e) => {
                    tracing::error!(error = %e, "Purge des tâches terminées");
                    return;
                }
            };
            let count = doomed.len() as i64;
            if doomed.is_empty() {
                break;
            }

            for (root,) in &doomed {
                match Self::purge_tree(state, *root).await {
                    Ok(n) => total += n,
                    Err(e) => {
                        tracing::error!(error = %e, "Purge d'une arborescence de tâches");
                        return;
                    }
                }
            }

            if count < BATCH {
                break;
            }
        }

        if total > 0 {
            tracing::info!(
                purged = total, retention_days = days,
                "Purge des tâches terminées au-delà de la rétention"
            );
        }
    }

    /// Tombstones every task in `root`'s subtree (owner-scoped, for tasks_delta)
    /// and deletes the root, whose FK cascade removes the rest. Returns how many
    /// rows were tombstoned. A root already removed by an earlier sibling's
    /// cascade yields an empty subtree and is a no-op.
    async fn purge_tree(state: &AppState, root: Uuid) -> Result<u64, sqlx::Error> {
        let subtree: Vec<(Uuid, Uuid)> = state
            .db
            .fetch_all_as(
                r#"
                WITH RECURSIVE sub AS (
                    SELECT id, owner_id FROM tasks.tasks WHERE id = $1
                    UNION ALL
                    SELECT t.id, t.owner_id FROM tasks.tasks t JOIN sub ON t.parent_task_id = sub.id
                )
                SELECT id, owner_id FROM sub
                "#,
                params![root],
            )
            .await?;
        if subtree.is_empty() {
            return Ok(0);
        }

        let mut tx = state.db.begin().await?;
        for (id, owner) in &subtree {
            let seq = sync::next_task_seq(&mut tx).await?;
            kubuno_db::journal::record_tombstone(&mut tx, sync::TASK_TOMBSTONES, *id, *owner, seq).await?;
        }
        tx.execute("DELETE FROM tasks.tasks WHERE id = $1", params![root]).await?;
        tx.commit().await?;
        Ok(subtree.len() as u64)
    }
}
