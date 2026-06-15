use std::collections::HashMap;

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{header, HeaderName},
    Extension, Json,
};
use uuid::Uuid;

use crate::{
    errors::{Result, TasksError},
    middleware::TasksUser,
    models::task::Task,
    services::{board_service::BoardService, icalendar_service::ICalendarService},
    state::AppState,
};

pub async fn export_board_ics(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(board_id): Path<Uuid>,
) -> Result<([(HeaderName, String); 2], String)> {
    let board = BoardService::get(board_id, user.id, &state.db).await?;
    let tasks = sqlx::query_as::<_, Task>(
        "SELECT * FROM tasks.tasks WHERE board_id = $1 ORDER BY created_at",
    )
    .bind(board_id)
    .fetch_all(&state.db)
    .await?;

    let ics = ICalendarService::board_to_ics(&board.title, &tasks);
    Ok((
        [
            (header::CONTENT_TYPE, "text/calendar; charset=utf-8".to_string()),
            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}.ics\"", board.title)),
        ],
        ics,
    ))
}

/// Importe un flux iCalendar (VTODO) dans un board. Deux passes : insertion des
/// tâches puis résolution des liens parent (RELATED-TO) et des labels (CATEGORIES).
pub async fn import_ics(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(board_id): Path<Uuid>,
    body: Bytes,
) -> Result<Json<serde_json::Value>> {
    BoardService::assert_access(board_id, user.id, "write", &state.db).await?;

    let content = std::str::from_utf8(&body)
        .map_err(|_| TasksError::Validation("corps non-UTF8".into()))?;
    let todos = ICalendarService::parse_vtodo(content)?;

    let mut tx = state.db.begin().await?;
    // uid (iCal) → id interne, pour résoudre les parents en 2e passe.
    let mut uid_to_id: HashMap<String, Uuid> = HashMap::new();
    // titre du label → id, pour réutiliser les labels du board.
    let mut label_cache: HashMap<String, Uuid> = HashMap::new();

    for todo in &todos {
        let reminders = serde_json::json!([]);
        let id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO tasks.tasks
                (board_id, owner_id, title, description, status, priority,
                 percent_complete, due_at, start_at, completed_at, rrule, reminders, ical_uid)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
            ON CONFLICT (ical_uid) DO UPDATE
                SET title = EXCLUDED.title, description = EXCLUDED.description,
                    status = EXCLUDED.status, priority = EXCLUDED.priority,
                    percent_complete = EXCLUDED.percent_complete, due_at = EXCLUDED.due_at,
                    start_at = EXCLUDED.start_at, completed_at = EXCLUDED.completed_at,
                    sequence = tasks.tasks.sequence + 1, etag = md5(random()::text)
            RETURNING id
            "#,
        )
        .bind(board_id)
        .bind(user.id)
        .bind(&todo.summary)
        .bind(&todo.description)
        .bind(&todo.status)
        .bind(todo.priority)
        .bind(todo.percent_complete)
        .bind(todo.due_at)
        .bind(todo.start_at)
        .bind(todo.completed_at)
        .bind(&todo.rrule)
        .bind(&reminders)
        .bind(&todo.uid)
        .fetch_one(&mut *tx)
        .await?;
        uid_to_id.insert(todo.uid.clone(), id);

        // Labels (CATEGORIES).
        for cat in &todo.categories {
            let label_id = match label_cache.get(cat) {
                Some(lid) => *lid,
                None => {
                    let lid: Uuid = sqlx::query_scalar(
                        r#"
                        INSERT INTO tasks.labels (board_id, title)
                        VALUES ($1, $2)
                        ON CONFLICT (board_id, title) DO UPDATE SET title = EXCLUDED.title
                        RETURNING id
                        "#,
                    )
                    .bind(board_id)
                    .bind(cat)
                    .fetch_one(&mut *tx)
                    .await?;
                    label_cache.insert(cat.clone(), lid);
                    lid
                }
            };
            sqlx::query(
                "INSERT INTO tasks.task_labels (task_id, label_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(id)
            .bind(label_id)
            .execute(&mut *tx)
            .await?;
        }
    }

    // 2e passe : résolution des parents.
    for todo in &todos {
        if let Some(ref parent_uid) = todo.parent_uid {
            if let (Some(child), Some(parent)) =
                (uid_to_id.get(&todo.uid), uid_to_id.get(parent_uid))
            {
                sqlx::query("UPDATE tasks.tasks SET parent_task_id = $1 WHERE id = $2")
                    .bind(parent)
                    .bind(child)
                    .execute(&mut *tx)
                    .await?;
            }
        }
    }

    tx.commit().await?;
    Ok(Json(serde_json::json!({ "imported": todos.len() })))
}
