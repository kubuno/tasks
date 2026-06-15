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
    models::label::{CreateLabelDto, UpdateLabelDto},
    services::label_service::LabelService,
    state::AppState,
};

pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(board_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let labels = LabelService::list(board_id, user.id, &state.db).await?;
    Ok(Json(serde_json::json!({ "labels": labels })))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(board_id): Path<Uuid>,
    Json(dto): Json<CreateLabelDto>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    dto.validate().map_err(|e| TasksError::Validation(e.to_string()))?;
    let label = LabelService::create(board_id, user.id, dto, &state.db).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "label": label }))))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateLabelDto>,
) -> Result<Json<serde_json::Value>> {
    let label = LabelService::update(id, user.id, dto, &state.db).await?;
    Ok(Json(serde_json::json!({ "label": label })))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    LabelService::delete(id, user.id, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}
