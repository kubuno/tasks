//! What the instance allows, as the module's screens need to know it.
//!
//! The core's `/modules/tasks/config` deliberately hides instance-scoped
//! settings from accounts without the settings privilege — rightly so, it is the
//! administration console's data. But the board screen still has to know that
//! sharing is closed, and it has to know it for ORDINARY accounts. So the module
//! publishes the few decisions its own screens act on, and nothing else: the
//! same ones the server already enforces, so the interface never offers an
//! action the backend will refuse.

use axum::{extract::State, Extension, Json};

use crate::{errors::Result, middleware::TasksUser, state::AppState};

pub async fn instance_policy(
    State(state): State<AppState>,
    Extension(_user): Extension<TasksUser>,
) -> Result<Json<serde_json::Value>> {
    let c = state.instance();
    Ok(Json(serde_json::json!({
        "allow_board_sharing":  c.allow_board_sharing,
        "max_boards_per_user":  c.max_boards_per_user,
        "max_tasks_per_board":  c.max_tasks_per_board,
        "attachment_max_mb":    c.attachment_max_mb,
    })))
}
