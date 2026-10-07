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

/// This module's id, used as the token audience: a token minted for another
/// module does not validate here.
const MODULE_ID: &str = "tasks";

/// Middleware: authenticate the caller from the signed `X-Kubuno-Auth` token the
/// core mints with this module's internal secret (see `kubuno-modauth`).
///
/// The plain `X-Kubuno-User-*` headers are no longer trusted: any process able
/// to reach this module's loopback port could set them to impersonate any user,
/// administrators included. The token binds the identity to the module secret
/// and carries a short expiry, so a forged or replayed header is rejected.
pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> std::result::Result<Response, TasksError> {
    let token = req
        .headers()
        .get(kubuno_modauth::TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
        .ok_or(TasksError::Unauthorized)?;

    let user = kubuno_modauth::verify(
        state.settings.core.internal_secret.as_bytes(),
        token,
        MODULE_ID,
    )
    .map_err(|_| TasksError::Unauthorized)?;

    req.extensions_mut()
        .insert(TasksUser { id: user.id, role: user.role, email: user.email });
    Ok(next.run(req).await)
}
