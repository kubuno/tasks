use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use kubuno_db::dialect::Assign;
use kubuno_db::params;

use crate::{
    models::{board::Board, task::Task},
    services::icalendar_service::ICalendarService,
    state::AppState,
    sync,
};

pub fn caldav_router() -> Router<AppState> {
    Router::new()
        .route("/.well-known/caldav", any(well_known))
        .route("/caldav/:username/", any(user_principal))
        .route("/caldav/:username/:token/", any(board_collection))
        .route("/caldav/:username/:token/:uid", any(task_resource))
}

fn xml_response(status: StatusCode, body: impl Into<String>) -> Response {
    (
        status,
        [(axum::http::header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        body.into(),
    )
        .into_response()
}

const XML_MULTISTATUS_START: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<D:multistatus xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">"#;
const XML_MULTISTATUS_END: &str = "</D:multistatus>";

async fn well_known(method: Method) -> Response {
    match method.as_str() {
        "OPTIONS" => (
            StatusCode::OK,
            [("Allow", "OPTIONS, PROPFIND, GET"), ("DAV", "1, calendar-access")],
            "",
        )
            .into_response(),
        _ => (
            StatusCode::MOVED_PERMANENTLY,
            [(axum::http::header::LOCATION, "/caldav/")],
            "",
        )
            .into_response(),
    }
}

async fn user_principal(
    method: Method,
    State(_state): State<AppState>,
    Path(username): Path<String>,
) -> Response {
    match method.as_str() {
        "OPTIONS" => (
            StatusCode::OK,
            [("Allow", "OPTIONS, GET, HEAD, PROPFIND, REPORT"), ("DAV", "1, calendar-access")],
        )
            .into_response(),
        "PROPFIND" => {
            let body = format!(
                r#"{XML_MULTISTATUS_START}
  <D:response>
    <D:href>/caldav/{username}/</D:href>
    <D:propstat>
      <D:prop>
        <D:displayname>{username}</D:displayname>
        <D:resourcetype><D:principal/><D:collection/></D:resourcetype>
        <C:calendar-home-set><D:href>/caldav/{username}/</D:href></C:calendar-home-set>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
{XML_MULTISTATUS_END}"#
            );
            xml_response(StatusCode::MULTI_STATUS, body)
        }
        _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
    }
}

// ── Collection = board (liste de VTODO) ───────────────────────────────────────

async fn board_collection(
    method: Method,
    State(state): State<AppState>,
    Path((username, token)): Path<(String, String)>,
) -> Response {
    match method.as_str() {
        "OPTIONS" => (
            StatusCode::OK,
            [
                ("Allow", "OPTIONS, GET, HEAD, PROPFIND, REPORT, PUT, DELETE"),
                ("DAV", "1, calendar-access"),
            ],
        )
            .into_response(),
        "PROPFIND" => {
            let board = match state
                .db
                .fetch_optional_as::<Board>(
                    "SELECT * FROM tasks.boards WHERE caldav_token = $1",
                    params![&token],
                )
                .await
            {
                Ok(Some(b)) => b,
                Ok(None) => return StatusCode::NOT_FOUND.into_response(),
                Err(e) => {
                    tracing::error!(error = %e, "CalDAV PROPFIND DB error");
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            };

            let body = format!(
                r#"{XML_MULTISTATUS_START}
  <D:response>
    <D:href>/caldav/{username}/{token}/</D:href>
    <D:propstat>
      <D:prop>
        <D:displayname>{}</D:displayname>
        <D:resourcetype><D:collection/><C:calendar/></D:resourcetype>
        <C:supported-calendar-component-set><C:comp name="VTODO"/></C:supported-calendar-component-set>
        <C:calendar-color>{}</C:calendar-color>
        <D:getctag>{}</D:getctag>
        <D:sync-token>{}</D:sync-token>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
{XML_MULTISTATUS_END}"#,
                board.title, board.color, board.ctag, board.ctag
            );
            xml_response(StatusCode::MULTI_STATUS, body)
        }
        "REPORT" => {
            let board_id = match state
                .db
                .fetch_optional_scalar::<uuid::Uuid>(
                    "SELECT id FROM tasks.boards WHERE caldav_token = $1",
                    params![&token],
                )
                .await
            {
                Ok(Some(id)) => id,
                Ok(None) => return StatusCode::NOT_FOUND.into_response(),
                Err(e) => {
                    tracing::error!(error = %e, "CalDAV REPORT DB error");
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            };

            let tasks = match state
                .db
                .fetch_all_as::<Task>("SELECT * FROM tasks.tasks WHERE board_id = $1", params![board_id])
                .await
            {
                Ok(t) => t,
                Err(e) => {
                    tracing::error!(error = %e, "CalDAV REPORT tasks DB error");
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            };

            let mut responses = String::new();
            for task in &tasks {
                let ics = ICalendarService::task_to_ics(task, &[]);
                let escaped = ics.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
                responses.push_str(&format!(
                    r#"  <D:response>
    <D:href>/caldav/{username}/{token}/{}.ics</D:href>
    <D:propstat>
      <D:prop>
        <D:getetag>{}</D:getetag>
        <C:calendar-data>{}</C:calendar-data>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
"#,
                    task.ical_uid, task.etag, escaped
                ));
            }
            let body = format!("{XML_MULTISTATUS_START}\n{responses}{XML_MULTISTATUS_END}");
            xml_response(StatusCode::MULTI_STATUS, body)
        }
        _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
    }
}

// ── Ressource = une tâche (VTODO) ─────────────────────────────────────────────

async fn task_resource(
    method: Method,
    State(state): State<AppState>,
    Path((_username, token, uid_with_ext)): Path<(String, String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let uid = uid_with_ext.trim_end_matches(".ics");

    match method.as_str() {
        "GET" | "HEAD" => {
            match state
                .db
                .fetch_optional_as::<Task>(
                    r#"
                    SELECT t.* FROM tasks.tasks t
                    JOIN tasks.boards b ON b.id = t.board_id
                    WHERE b.caldav_token = $1 AND t.ical_uid = $2
                    "#,
                    params![token, uid],
                )
                .await
            {
                Ok(Some(task)) => {
                    let ics = ICalendarService::task_to_ics(&task, &[]);
                    (
                        StatusCode::OK,
                        [
                            (axum::http::header::CONTENT_TYPE, "text/calendar; charset=utf-8"),
                            (axum::http::header::ETAG, task.etag.as_str()),
                        ],
                        ics,
                    )
                        .into_response()
                }
                Ok(None) => StatusCode::NOT_FOUND.into_response(),
                Err(e) => {
                    tracing::error!(error = %e, "CalDAV GET error");
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
        "DELETE" => delete_task(&state, &token, uid).await,
        "PUT" => put_task(&state, &token, uid, &headers, &body).await,
        "OPTIONS" => (
            StatusCode::OK,
            [
                ("Allow", "OPTIONS, GET, HEAD, PUT, DELETE, PROPFIND"),
                ("DAV", "1, calendar-access"),
            ],
        )
            .into_response(),
        _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
    }
}

/// The `USING`-join delete is PostgreSQL-only, so the task is resolved first and
/// then deleted by id — which also lets the deletion take a change_seq and write
/// its tombstone (what the old AFTER DELETE trigger did).
async fn delete_task(state: &AppState, token: &str, uid: &str) -> Response {
    let found = state
        .db
        .fetch_optional_as::<(uuid::Uuid, uuid::Uuid)>(
            r#"
            SELECT t.id, t.owner_id FROM tasks.tasks t
            JOIN tasks.boards b ON b.id = t.board_id
            WHERE b.caldav_token = $1 AND t.ical_uid = $2
            "#,
            params![token, uid],
        )
        .await;

    let (task_id, owner_id) = match found {
        Ok(Some(v)) => v,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!(error = %e, "CalDAV DELETE lookup error");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let result = async {
        let mut tx = state.db.begin().await?;
        let seq = sync::next_task_seq(&mut tx).await?;
        tx.execute("DELETE FROM tasks.tasks WHERE id = $1", params![task_id]).await?;
        kubuno_db::journal::record_tombstone(&mut tx, sync::TASK_TOMBSTONES, task_id, owner_id, seq).await?;
        tx.commit().await
    }
    .await;

    match result {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            tracing::error!(error = %e, "CalDAV DELETE error");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn put_task(
    state: &AppState,
    token: &str,
    uid: &str,
    headers: &HeaderMap,
    body: &Bytes,
) -> Response {
    let (board_id, owner_id) = match state
        .db
        .fetch_optional_as::<(uuid::Uuid, uuid::Uuid)>(
            "SELECT id, owner_id FROM tasks.boards WHERE caldav_token = $1",
            params![token],
        )
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!(error = %e, "CalDAV PUT board lookup error");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let content = match std::str::from_utf8(body) {
        Ok(c) => c,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let todos = match ICalendarService::parse_vtodo(content) {
        Ok(t) => t,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let todo = match todos.into_iter().next() {
        Some(t) => t,
        None => return StatusCode::BAD_REQUEST.into_response(),
    };

    let ical_uid = if todo.uid.is_empty() { uid.to_string() } else { todo.uid.clone() };

    // If-None-Match: * — création seulement.
    let if_none_match = headers
        .get(axum::http::header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok());
    if if_none_match == Some("*") {
        let exists: Option<uuid::Uuid> = state
            .db
            .fetch_optional_scalar("SELECT id FROM tasks.tasks WHERE ical_uid = $1", params![&ical_uid])
            .await
            .ok()
            .flatten();
        if exists.is_some() {
            return StatusCode::PRECONDITION_FAILED.into_response();
        }
    }

    let etag = sync::new_tag();
    let backend = state.db.backend();
    // Upsert on ical_uid; the DO UPDATE mirrors the fields the old statement set,
    // with `sequence` bumped by an expression and the new etag / change_seq from
    // the incoming row. `md5(random()::text)` is gone: the etag is minted here.
    let clause = backend.upsert(
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
            Assign::Incoming("rrule"),
            Assign::Expr { col: "sequence", expr: "{cur} + 1" },
            Assign::Incoming("etag"),
            Assign::Incoming("change_seq"),
        ],
    );
    let reminders = serde_json::json!([]);
    let empty_files: Vec<uuid::Uuid> = Vec::new();

    let result = async {
        let mut tx = state.db.begin().await?;
        let seq = sync::next_task_seq(&mut tx).await?;
        let sql = format!(
            "INSERT INTO tasks.tasks
               (id, board_id, owner_id, title, description, status, priority, percent_complete,
                due_at, start_at, completed_at, rrule, reminders, ical_uid, etag, change_seq, linked_file_ids)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17){clause}"
        );
        tx.execute(
            &sql,
            params![
                kubuno_db::new_id(), board_id, owner_id, todo.summary, todo.description, todo.status,
                todo.priority, todo.percent_complete, todo.due_at, todo.start_at, todo.completed_at,
                todo.rrule, reminders, ical_uid, etag.clone(), seq, empty_files
            ],
        )
        .await?;
        tx.commit().await.map(|_| etag)
    }
    .await;

    match result {
        Ok(etag) => (StatusCode::CREATED, [(axum::http::header::ETAG, etag)], "").into_response(),
        Err(e) => {
            tracing::error!(error = %e, "CalDAV PUT upsert error");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
