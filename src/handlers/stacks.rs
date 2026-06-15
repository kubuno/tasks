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
    models::stack::{CreateStackDto, ReorderStacksDto, UpdateStackDto},
    services::stack_service::StackService,
    state::AppState,
};

pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(board_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let stacks = StackService::list(board_id, user.id, &state.db).await?;
    Ok(Json(serde_json::json!({ "stacks": stacks })))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(board_id): Path<Uuid>,
    Json(dto): Json<CreateStackDto>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    dto.validate().map_err(|e| TasksError::Validation(e.to_string()))?;
    let stack = StackService::create(board_id, user.id, dto, &state.db).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "stack": stack }))))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateStackDto>,
) -> Result<Json<serde_json::Value>> {
    let stack = StackService::update(id, user.id, dto, &state.db).await?;
    Ok(Json(serde_json::json!({ "stack": stack })))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    StackService::delete(id, user.id, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn reorder(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(board_id): Path<Uuid>,
    Json(dto): Json<ReorderStacksDto>,
) -> Result<Json<serde_json::Value>> {
    let stacks = StackService::reorder(board_id, user.id, dto.ordered_ids, &state.db).await?;
    Ok(Json(serde_json::json!({ "stacks": stacks })))
}
