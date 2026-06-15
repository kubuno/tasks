use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use uuid::Uuid;
use validator::Validate;

use crate::{
    errors::{Result, TasksError},
    middleware::TasksUser,
    models::attachment::CreateAttachmentDto,
    services::attachment_service::AttachmentService,
    state::AppState,
};

pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let attachments = AttachmentService::list(task_id, user.id, &state.db).await?;
    Ok(Json(serde_json::json!({ "attachments": attachments })))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(task_id): Path<Uuid>,
    Json(dto): Json<CreateAttachmentDto>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    dto.validate().map_err(|e| TasksError::Validation(e.to_string()))?;
    let attachment = AttachmentService::create(task_id, user.id, dto, &state.db).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "attachment": attachment }))))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    AttachmentService::delete(id, user.id, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}
