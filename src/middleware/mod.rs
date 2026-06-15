use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::{errors::TasksError, state::AppState};

/// Utilisateur extrait des headers injectés par le core.
#[derive(Debug, Clone)]
pub struct TasksUser {
    pub id:    Uuid,
    pub role:  String,
    pub email: String,
}

/// Clé d'extension Axum pour stocker l'utilisateur dans la requête.
pub type TasksUserExt = axum::Extension<TasksUser>;

/// Middleware : extrait X-Kubuno-User-Id, X-Kubuno-User-Role, X-Kubuno-User-Email.
/// Ces headers sont injectés par le proxy du core — on leur fait confiance.
pub async fn require_auth(
    State(_state): State<AppState>,
    mut req: Request,
    next: Next,
) -> std::result::Result<Response, TasksError> {
    let user_id = req
        .headers()
        .get("x-kubuno-user-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or(TasksError::Unauthorized)?;

    let role = req
        .headers()
        .get("x-kubuno-user-role")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("user")
        .to_string();

    let email = req
        .headers()
        .get("x-kubuno-user-email")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    req.extensions_mut()
        .insert(TasksUser { id: user_id, role, email });
    Ok(next.run(req).await)
}
