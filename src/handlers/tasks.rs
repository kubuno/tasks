use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderName, StatusCode},
    Extension, Json,
};
use uuid::Uuid;
use validator::Validate;

use crate::{
    errors::{Result, TasksError},
    events::publisher,
    middleware::TasksUser,
    models::task::{CreateTaskDto, MoveTaskDto, MoveToBoardDto, TasksQuery, UpdateTaskDto},
    services::{icalendar_service::ICalendarService, task_service::TaskService},
    state::AppState,
};

/// Émet un événement vers le core sans bloquer la réponse.
fn emit(state: &AppState, kind: &'static str, task_id: Uuid, user_id: Uuid) {
    let st = state.clone();
    tokio::spawn(async move {
        match kind {
            "created"   => publisher::publish_task_created(&st, task_id, user_id).await,
            "updated"   => publisher::publish_task_updated(&st, task_id, user_id).await,
            "deleted"   => publisher::publish_task_deleted(&st, task_id, user_id).await,
            "completed" => publisher::publish_task_completed(&st, task_id, user_id).await,
            _ => {}
        }
    });
}

pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Query(q): Query<TasksQuery>,
) -> Result<Json<serde_json::Value>> {
    let tasks = TaskService::list(user.id, &q, &state.db).await?;
    Ok(Json(serde_json::json!({ "tasks": tasks })))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Json(dto): Json<CreateTaskDto>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    dto.validate().map_err(|e| TasksError::Validation(e.to_string()))?;
    let task = TaskService::create(user.id, dto, &state.db).await?;
    emit(&state, "created", task.task.id, user.id);
    if task.task.status == "done" {
        emit(&state, "completed", task.task.id, user.id);
    }
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "task": task }))))
}

pub async fn get(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let task = TaskService::get_with_meta(id, user.id, &state.db).await?;
    Ok(Json(serde_json::json!({ "task": task })))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateTaskDto>,
) -> Result<Json<serde_json::Value>> {
    dto.validate().map_err(|e| TasksError::Validation(e.to_string()))?;
    let task = TaskService::update(id, user.id, dto, &state.db).await?;
    emit(&state, "updated", id, user.id);
    if task.task.status == "done" {
        emit(&state, "completed", id, user.id);
    }
    Ok(Json(serde_json::json!({ "task": task })))
}

pub async fn delete(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode> {
    TaskService::delete(id, user.id, &state.db).await?;
    emit(&state, "deleted", id, user.id);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn move_task(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(id): Path<Uuid>,
    Json(dto): Json<MoveTaskDto>,
) -> Result<Json<serde_json::Value>> {
    let task = TaskService::move_task(id, user.id, dto, &state.db).await?;
    emit(&state, "updated", id, user.id);
    Ok(Json(serde_json::json!({ "task": task })))
}

pub async fn complete(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let task = TaskService::complete(id, user.id, &state.db).await?;
    emit(&state, "updated", id, user.id);
    emit(&state, "completed", id, user.id);
    Ok(Json(serde_json::json!({ "task": task })))
}

/// Déplace une ou plusieurs tâches vers un autre board.
pub async fn move_to_board(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Json(dto): Json<MoveToBoardDto>,
) -> Result<Json<serde_json::Value>> {
    dto.validate().map_err(|e| TasksError::Validation(e.to_string()))?;
    let moved = TaskService::move_to_board(user.id, dto, &state.db).await?;
    for id in &moved {
        emit(&state, "updated", *id, user.id);
    }
    Ok(Json(serde_json::json!({ "moved": moved })))
}

pub async fn list_subtasks(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let subtasks = TaskService::list_subtasks(id, user.id, &state.db).await?;
    Ok(Json(serde_json::json!({ "tasks": subtasks })))
}

pub async fn create_subtask(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(id): Path<Uuid>,
    Json(mut dto): Json<CreateTaskDto>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    dto.validate().map_err(|e| TasksError::Validation(e.to_string()))?;
    let board_id = TaskService::board_of_task(id, &state.db).await?;
    if dto.board_id != board_id {
        return Err(TasksError::Validation("board_id incohérent avec la tâche parente".into()));
    }
    dto.parent_task_id = Some(id);
    let task = TaskService::create(user.id, dto, &state.db).await?;
    emit(&state, "created", task.task.id, user.id);
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "task": task }))))
}

pub async fn export_ics(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(id): Path<Uuid>,
) -> Result<([(HeaderName, String); 2], String)> {
    let meta = TaskService::get_with_meta(id, user.id, &state.db).await?;
    let categories: Vec<String> = meta.labels.iter().map(|l| l.title.clone()).collect();
    let ics = ICalendarService::task_to_ics(&meta.task, &categories);
    Ok((
        [
            (header::CONTENT_TYPE, "text/calendar; charset=utf-8".to_string()),
            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}.ics\"", meta.task.ical_uid)),
        ],
        ics,
    ))
}

// ── Assignés ────────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct AssigneeBody {
    pub user_id: Uuid,
}

pub async fn list_assignees(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let assignees = TaskService::list_assignees(id, user.id, &state.db).await?;
    Ok(Json(serde_json::json!({ "assignees": assignees })))
}

pub async fn add_assignee(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(id): Path<Uuid>,
    Json(body): Json<AssigneeBody>,
) -> Result<StatusCode> {
    TaskService::add_assignee(id, user.id, body.user_id, &state.db).await?;
    emit(&state, "updated", id, user.id);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_assignee(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path((id, assignee)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    TaskService::remove_assignee(id, user.id, assignee, &state.db).await?;
    emit(&state, "updated", id, user.id);
    Ok(StatusCode::NO_CONTENT)
}

// ── Labels sur une tâche ──────────────────────────────────────────────────────────

pub async fn add_label(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path((id, label_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    TaskService::add_label(id, user.id, label_id, &state.db).await?;
    emit(&state, "updated", id, user.id);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_label(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path((id, label_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode> {
    TaskService::remove_label(id, user.id, label_id, &state.db).await?;
    emit(&state, "updated", id, user.id);
    Ok(StatusCode::NO_CONTENT)
}
