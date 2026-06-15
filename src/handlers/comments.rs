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
    models::comment::{CreateCommentDto, UpdateCommentDto},
    services::comment_service::CommentService,
    state::AppState,
};

pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let comments = CommentService::list(task_id, user.id, &state.db).await?;
    Ok(Json(serde_json::json!({ "comments": comments })))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(task_id): Path<Uuid>,
    Json(dto): Json<CreateCommentDto>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    dto.validate().map_err(|e| TasksError::Validation(e.to_string()))?;
    let comment = CommentService::create(task_id, user.id, dto, &state.db).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "comment": comment }))))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateCommentDto>,
) -> Result<Json<serde_json::Value>> {
    dto.validate().map_err(|e| TasksError::Validation(e.to_string()))?;
    let comment = CommentService::update(id, user.id, dto, &state.db).await?;
    Ok(Json(serde_json::json!({ "comment": comment })))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    CommentService::delete(id, user.id, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}
