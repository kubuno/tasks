use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};

use crate::{
    models::{board::Board, task::Task},
    services::icalendar_service::ICalendarService,
    state::AppState,
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
            let board = match sqlx::query_as::<_, Board>(
                "SELECT * FROM tasks.boards WHERE caldav_token = $1",
            )
            .bind(&token)
            .fetch_optional(&state.db)
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
            let board_id = match sqlx::query_scalar::<_, uuid::Uuid>(
                "SELECT id FROM tasks.boards WHERE caldav_token = $1",
            )
            .bind(&token)
            .fetch_optional(&state.db)
            .await
            {
                Ok(Some(id)) => id,
                Ok(None) => return StatusCode::NOT_FOUND.into_response(),
                Err(e) => {
                    tracing::error!(error = %e, "CalDAV REPORT DB error");
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            };

            let tasks = match sqlx::query_as::<_, Task>(
                "SELECT * FROM tasks.tasks WHERE board_id = $1",
            )
            .bind(board_id)
            .fetch_all(&state.db)
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
            match sqlx::query_as::<_, Task>(
                r#"
                SELECT t.* FROM tasks.tasks t
                JOIN tasks.boards b ON b.id = t.board_id
                WHERE b.caldav_token = $1 AND t.ical_uid = $2
                "#,
            )
            .bind(&token)
            .bind(uid)
            .fetch_optional(&state.db)
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
        "DELETE" => {
            match sqlx::query(
                r#"
                DELETE FROM tasks.tasks t
                USING tasks.boards b
                WHERE t.board_id = b.id AND b.caldav_token = $1 AND t.ical_uid = $2
                "#,
            )
            .bind(&token)
            .bind(uid)
            .execute(&state.db)
            .await
            {
                Ok(r) if r.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
                Ok(_) => StatusCode::NOT_FOUND.into_response(),
                Err(e) => {
                    tracing::error!(error = %e, "CalDAV DELETE error");
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
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

async fn put_task(
    state: &AppState,
    token: &str,
    uid: &str,
    headers: &HeaderMap,
    body: &Bytes,
) -> Response {
    // Résolution du board via le token.
    let (board_id, owner_id) = match sqlx::query_as::<_, (uuid::Uuid, uuid::Uuid)>(
        "SELECT id, owner_id FROM tasks.boards WHERE caldav_token = $1",
    )
    .bind(token)
    .fetch_optional(&state.db)
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

    // L'UID stocké : celui du corps si présent, sinon le nom de ressource.
    let ical_uid = if todo.uid.is_empty() { uid.to_string() } else { todo.uid.clone() };

    // Gestion If-None-Match: * (création seulement).
    let if_none_match = headers
        .get(axum::http::header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok());
    if if_none_match == Some("*") {
        let exists: Option<uuid::Uuid> =
            sqlx::query_scalar("SELECT id FROM tasks.tasks WHERE ical_uid = $1")
                .bind(&ical_uid)
                .fetch_optional(&state.db)
                .await
                .ok()
                .flatten();
        if exists.is_some() {
            return StatusCode::PRECONDITION_FAILED.into_response();
        }
    }

    let reminders = serde_json::json!([]);
    let result = sqlx::query_scalar::<_, String>(
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
                rrule = EXCLUDED.rrule, sequence = tasks.tasks.sequence + 1,
                etag = md5(random()::text)
        RETURNING etag
        "#,
    )
    .bind(board_id)
    .bind(owner_id)
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
    .bind(&ical_uid)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok(etag) => (
            StatusCode::CREATED,
            [(axum::http::header::ETAG, etag)],
            "",
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "CalDAV PUT upsert error");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
