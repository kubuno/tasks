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
    models::board::{CreateBoardDto, ShareBoardDto, UpdateBoardDto},
    models::comment::{CreateCommentDto, UpdateCommentDto},
    services::{board_comment_service::BoardCommentService, board_service::BoardService},
    state::AppState,
};

pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
) -> Result<Json<serde_json::Value>> {
    let boards = BoardService::list(user.id, &state.db).await?;
    Ok(Json(serde_json::json!({ "boards": boards })))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Json(dto): Json<CreateBoardDto>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    dto.validate().map_err(|e| TasksError::Validation(e.to_string()))?;
    let board = BoardService::create(user.id, dto, &state.db).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "board": board }))))
}

pub async fn get(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let board = BoardService::get(id, user.id, &state.db).await?;
    let shares = BoardService::list_shares(id, user.id, &state.db).await?;
    Ok(Json(serde_json::json!({ "board": board, "shares": shares })))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateBoardDto>,
) -> Result<Json<serde_json::Value>> {
    let board = BoardService::update(id, user.id, dto, &state.db).await?;
    Ok(Json(serde_json::json!({ "board": board })))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    BoardService::delete(id, user.id, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn share(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<ShareBoardDto>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    let share = BoardService::share(id, user.id, dto, &state.db).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "share": share }))))
}

pub async fn unshare(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path((id, shared_with)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    BoardService::unshare(id, user.id, shared_with, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Commentaires de board ─────────────────────────────────────────────────────

pub async fn list_comments(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let comments = BoardCommentService::list(id, user.id, &state.db).await?;
    Ok(Json(serde_json::json!({ "comments": comments })))
}

pub async fn create_comment(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<CreateCommentDto>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    use validator::Validate;
    dto.validate().map_err(|e| TasksError::Validation(e.to_string()))?;
    let comment = BoardCommentService::create(id, user.id, dto, &state.db).await?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "comment": comment }))))
}

pub async fn update_comment(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(cid): Path<Uuid>,
    Json(dto): Json<UpdateCommentDto>,
) -> Result<Json<serde_json::Value>> {
    use validator::Validate;
    dto.validate().map_err(|e| TasksError::Validation(e.to_string()))?;
    let comment = BoardCommentService::update(cid, user.id, dto, &state.db).await?;
    Ok(Json(serde_json::json!({ "comment": comment })))
}

pub async fn delete_comment(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(cid): Path<Uuid>,
) -> Result<StatusCode> {
    BoardCommentService::delete(cid, user.id, &state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}
