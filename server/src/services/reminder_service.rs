use chrono::Utc;
use kubuno_db::params;
use std::sync::Arc;
use uuid::Uuid;

use crate::{errors::Result, state::AppState};

pub struct ReminderService;

impl ReminderService {
    /// Worker qui vérifie les rappels dus toutes les minutes.
    pub async fn run_worker(state: Arc<AppState>) {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            if let Err(e) = Self::process_due_reminders(&state).await {
                tracing::error!(error = %e, "Erreur traitement des rappels");
            }
        }
    }

    async fn process_due_reminders(state: &AppState) -> Result<()> {
        // NOW() is bound from Rust: the three engines spell it differently and
        // SQLite has no time zone.
        let due: Vec<(Uuid, Uuid, String)> = state
            .db
            .fetch_all_as(
                r#"
                SELECT id, user_id, channel
                FROM tasks.scheduled_reminders
                WHERE sent = FALSE AND remind_at <= $1
                ORDER BY remind_at
                LIMIT 100
                "#,
                params![Utc::now()],
            )
            .await?;

        for (reminder_id, user_id, channel) in due {
            tracing::info!(reminder_id = %reminder_id, user_id = %user_id, channel = %channel, "Envoi rappel tâche");
            state
                .db
                .execute(
                    "UPDATE tasks.scheduled_reminders SET sent = TRUE, sent_at = $1 WHERE id = $2",
                    params![Utc::now(), reminder_id],
                )
                .await?;
            // TODO: livraison effective (WebSocket/push/email) via le core.
        }
        Ok(())
    }
}
