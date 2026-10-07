use crate::state::AppState;
use serde_json::json;
use uuid::Uuid;

pub async fn publish_task_created(state: &AppState, task_id: Uuid, user_id: Uuid) {
    publish(state, "TaskCreated", task_id, user_id).await;
}

pub async fn publish_task_updated(state: &AppState, task_id: Uuid, user_id: Uuid) {
    publish(state, "TaskUpdated", task_id, user_id).await;
}

pub async fn publish_task_deleted(state: &AppState, task_id: Uuid, user_id: Uuid) {
    publish(state, "TaskDeleted", task_id, user_id).await;
}

pub async fn publish_task_completed(state: &AppState, task_id: Uuid, user_id: Uuid) {
    publish(state, "TaskCompleted", task_id, user_id).await;
}

async fn publish(state: &AppState, event_type: &str, task_id: Uuid, user_id: Uuid) {
    let payload = json!({
        "type": event_type,
        "payload": {
            "task_id":   task_id,
            "user_id":   user_id,
            "module_id": "tasks",
        }
    });
    send_to_core(state, &payload).await;
}

async fn send_to_core(state: &AppState, payload: &serde_json::Value) {
    let url = format!("{}/internal/events/publish", state.settings.core.url);
    match reqwest::Client::new()
        .post(&url)
        .header("X-Internal-Secret", &state.settings.core.internal_secret)
        .json(payload)
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => {}
        Ok(r) => tracing::warn!(status = %r.status(), "Publish event: réponse inattendue"),
        Err(e) => tracing::warn!(error = %e, "Publish event: erreur réseau"),
    }
}
