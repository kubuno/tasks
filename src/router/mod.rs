use axum::{
    middleware,
    routing::{delete, get, patch, post, put},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    handlers::{attachments, boards, caldav, comments, health, import_export, labels, stacks, tasks},
    middleware::require_auth,
    state::AppState,
};

pub fn build(state: AppState) -> Router {
    let authed = Router::new()
        // Boards
        .route("/boards",                   get(boards::list).post(boards::create))
        .route("/boards/:id",               get(boards::get).patch(boards::update).delete(boards::delete))
        .route("/boards/:id/share",         post(boards::share))
        .route("/boards/:id/share/:uid",    delete(boards::unshare))
        .route("/boards/:id/comments",      get(boards::list_comments).post(boards::create_comment))
        .route("/board-comments/:id",       patch(boards::update_comment).delete(boards::delete_comment))
        .route("/boards/:id/export",        get(import_export::export_board_ics))
        .route("/boards/:id/import",        post(import_export::import_ics))
        // Stacks (colonnes Kanban)
        .route("/boards/:id/stacks",         get(stacks::list).post(stacks::create))
        .route("/boards/:id/stacks/reorder", post(stacks::reorder))
        .route("/stacks/:id",                patch(stacks::update).delete(stacks::delete))
        // Labels
        .route("/boards/:id/labels",        get(labels::list).post(labels::create))
        .route("/labels/:id",               patch(labels::update).delete(labels::delete))
        // Tasks (cartes) + collections intelligentes
        .route("/move-tasks",               post(tasks::move_to_board))
        .route("/tasks",                    get(tasks::list).post(tasks::create))
        .route("/tasks/:id",                get(tasks::get).patch(tasks::update).delete(tasks::delete))
        .route("/tasks/:id/move",           post(tasks::move_task))
        .route("/tasks/:id/complete",       post(tasks::complete))
        .route("/tasks/:id/subtasks",       get(tasks::list_subtasks).post(tasks::create_subtask))
        .route("/tasks/:id/ics",            get(tasks::export_ics))
        .route("/tasks/:id/assignees",      get(tasks::list_assignees).post(tasks::add_assignee))
        .route("/tasks/:id/assignees/:uid", delete(tasks::remove_assignee))
        .route("/tasks/:id/labels/:lid",    put(tasks::add_label).delete(tasks::remove_label))
        // Comments
        .route("/tasks/:id/comments",       get(comments::list).post(comments::create))
        .route("/comments/:id",             patch(comments::update).delete(comments::delete))
        // Attachments
        .route("/tasks/:id/attachments",    get(attachments::list).post(attachments::create))
        .route("/attachments/:id",          delete(attachments::delete))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth))
        .with_state(state.clone());

    let system = Router::new()
        .route("/health", get(health::health))
        .with_state(state.clone());

    let caldav_routes = caldav::caldav_router().with_state(state);

    Router::new()
        .merge(system)
        .nest("/", authed)
        .merge(caldav_routes)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}
