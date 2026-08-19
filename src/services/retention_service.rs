//! Retention of completed tasks.
//!
//! An administration that sets a retention is asking the instance to stop
//! keeping a done-list nobody reads — a data-minimisation decision, so it is the
//! server that applies it and not whichever client happens to be open. The knob
//! is `completed_task_retention_days`; left at `0` (the default) this worker does
//! nothing at all, and says nothing either.
//!
//! What is deliberately NOT purged: a task that is done but whose subtasks are
//! not. Deleting it would cascade over work still in progress (the subtask rows
//! reference the parent `ON DELETE CASCADE`), and "completed" is a claim about
//! the parent alone. A fully finished tree does go, subtasks included.

use chrono::{Duration, Utc};

use crate::state::AppState;

/// How often the cleaner wakes up. A retention is expressed in days, so there is
/// nothing to gain from a tighter loop.
const SWEEP_INTERVAL: std::time::Duration = std::time::Duration::from_secs(6 * 3600);

/// Tasks deleted per pass. Bounds both the transaction and the surprise: a
/// freshly enabled retention on an old instance trims steadily instead of
/// locking the table once.
const BATCH: i64 = 500;

pub struct RetentionService;

impl RetentionService {
    /// Runs the sweep forever. First pass 5 minutes after startup, so a module
    /// restart never coincides with a bulk delete.
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

        let mut total: u64 = 0;
        loop {
            let deleted = sqlx::query(
                r#"
                DELETE FROM tasks.tasks
                WHERE id IN (
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
                )
                "#,
            )
            .bind(cutoff)
            .bind(BATCH)
            .execute(&state.db)
            .await;

            match deleted {
                Ok(res) => {
                    let n = res.rows_affected();
                    total += n;
                    if (n as i64) < BATCH {
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "Purge des tâches terminées");
                    return;
                }
            }
        }

        if total > 0 {
            tracing::info!(
                purged = total, retention_days = days,
                "Purge des tâches terminées au-delà de la rétention"
            );
        }
    }
}
