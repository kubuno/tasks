use std::collections::HashMap;

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{header, HeaderName},
    Extension, Json,
};
use kubuno_db::dialect::Assign;
use kubuno_db::params;
use uuid::Uuid;

use crate::{
    errors::{Result, TasksError},
    middleware::TasksUser,
    models::task::Task,
    services::{board_service::BoardService, icalendar_service::ICalendarService},
    state::AppState,
    sync,
};

pub async fn export_board_ics(
    State(state): State<AppState>,
    Extension(user): Extension<TasksUser>,
    Path(board_id): Path<Uuid>,
) -> Result<([(HeaderName, String); 2], String)> {
    let board = BoardService::get(board_id, user.id, &state.db).await?;
    let tasks = state
        .db
        .fetch_all_as::<Task>(
            "SELECT * FROM tasks.tasks WHERE board_id = $1 ORDER BY created_at",
            params![board_id],
        )
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

    let backend = state.db.backend();
    // Task upsert on ical_uid (rrule stays out of the DO UPDATE, as before).
    let task_clause = backend.upsert(
        "tasks.tasks",
        &["ical_uid"],
        &[
            Assign::Incoming("title"),
            Assign::Incoming("description"),
            Assign::Incoming("status"),
            Assign::Incoming("priority"),
            Assign::Incoming("percent_complete"),
            Assign::Incoming("due_at"),
            Assign::Incoming("start_at"),
            Assign::Incoming("completed_at"),
            Assign::Expr { col: "sequence", expr: "{cur} + 1" },
            Assign::Incoming("etag"),
            Assign::Incoming("change_seq"),
        ],
    );
    let label_clause = backend.upsert("labels", &["board_id", "title"], &[Assign::Incoming("title")]);
    let tl_ignore = backend.insert_ignore_prefix();
    let tl_nothing = backend.on_conflict_do_nothing(&["task_id", "label_id"]);

    let mut tx = state.db.begin().await?;
    let mut uid_to_id: HashMap<String, Uuid> = HashMap::new();
    let mut label_cache: HashMap<String, Uuid> = HashMap::new();
    let empty_files: Vec<Uuid> = Vec::new();

    for todo in &todos {
        let reminders = serde_json::json!([]);
        let etag = sync::new_tag();
        let seq = sync::next_task_seq(&mut tx).await?;
        let task_sql = format!(
            "INSERT INTO tasks.tasks
               (id, board_id, owner_id, title, description, status, priority, percent_complete,
                due_at, start_at, completed_at, rrule, reminders, ical_uid, etag, change_seq, linked_file_ids)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17){task_clause}"
        );
        let id: Uuid = kubuno_db::returning::insert_returning_scalar(
            &mut tx,
            &task_sql,
            params![
                kubuno_db::new_id(), board_id, user.id, &todo.summary, todo.description.as_deref(),
                &todo.status, todo.priority, todo.percent_complete, todo.due_at, todo.start_at,
                todo.completed_at, todo.rrule.as_deref(), reminders, &todo.uid, etag, seq,
                empty_files.clone()
            ],
            "id",
            "SELECT id FROM tasks.tasks WHERE ical_uid = $1",
            params![todo.uid.clone()],
        )
        .await?;
        uid_to_id.insert(todo.uid.clone(), id);

        // Labels (CATEGORIES). Each is a board child, so the board is touched.
        for cat in &todo.categories {
            let label_id = match label_cache.get(cat) {
                Some(lid) => *lid,
                None => {
                    let label_sql = format!(
                        "INSERT INTO tasks.labels (id, board_id, title) VALUES ($1, $2, $3){label_clause}"
                    );
                    let lid: Uuid = kubuno_db::returning::insert_returning_scalar(
                        &mut tx,
                        &label_sql,
                        params![kubuno_db::new_id(), board_id, cat.clone()],
                        "id",
                        "SELECT id FROM tasks.labels WHERE board_id = $1 AND title = $2",
                        params![board_id, cat.clone()],
                    )
                    .await?;
                    sync::touch_board(&mut tx, board_id).await?;
                    label_cache.insert(cat.clone(), lid);
                    lid
                }
            };
            tx.execute(
                &format!(
                    "INSERT {tl_ignore}INTO tasks.task_labels (task_id, label_id) VALUES ($1, $2){tl_nothing}"
                ),
                params![id, label_id],
            )
            .await?;
            sync::touch_task(&mut tx, id).await?;
        }
    }

    // 2e passe : résolution des parents (a task update → a fresh change_seq).
    for todo in &todos {
        if let Some(ref parent_uid) = todo.parent_uid {
            if let (Some(child), Some(parent)) =
                (uid_to_id.get(&todo.uid), uid_to_id.get(parent_uid))
            {
                let seq = sync::next_task_seq(&mut tx).await?;
                tx.execute(
                    "UPDATE tasks.tasks SET parent_task_id = $1, change_seq = $2 WHERE id = $3",
                    params![*parent, seq, *child],
                )
                .await?;
            }
        }
    }

    tx.commit().await?;
    Ok(Json(serde_json::json!({ "imported": todos.len() })))
}
