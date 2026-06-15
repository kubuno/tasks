use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::label::Label;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Task {
    pub id:               Uuid,
    pub board_id:         Uuid,
    pub stack_id:         Option<Uuid>,
    pub parent_task_id:   Option<Uuid>,
    pub owner_id:         Uuid,
    pub title:            String,
    pub description:      Option<String>,
    pub status:           String,
    pub priority:         i16,
    pub percent_complete: i16,
    pub due_at:           Option<DateTime<Utc>>,
    pub start_at:         Option<DateTime<Utc>>,
    pub completed_at:     Option<DateTime<Utc>>,
    pub all_day:          bool,
    pub color:            Option<String>,
    pub rrule:            Option<String>,
    pub reminders:        Value,
    pub ical_uid:         String,
    pub etag:             String,
    pub sequence:         i32,
    pub sort_order:       i32,
    pub position:         f64,
    pub linked_event_id:  Option<Uuid>,
    pub linked_file_ids:  Vec<Uuid>,
    pub created_at:       DateTime<Utc>,
    pub updated_at:       DateTime<Utc>,
}

/// Tâche enrichie de ses métadonnées (labels, assignés, compteurs).
#[derive(Debug, Clone, Serialize)]
pub struct TaskWithMeta {
    #[serde(flatten)]
    pub task:          Task,
    pub labels:        Vec<Label>,
    pub assignees:     Vec<Uuid>,
    pub subtask_count: i64,
    pub comment_count: i64,
}

#[derive(Debug, Deserialize, validator::Validate)]
pub struct CreateTaskDto {
    pub board_id:        Uuid,
    pub stack_id:        Option<Uuid>,
    pub parent_task_id:  Option<Uuid>,
    #[validate(length(min = 1, max = 500))]
    pub title:           String,
    pub description:     Option<String>,
    pub status:          Option<String>,
    #[validate(range(min = 0, max = 9))]
    pub priority:        Option<i16>,
    #[validate(range(min = 0, max = 100))]
    pub percent_complete: Option<i16>,
    pub due_at:          Option<DateTime<Utc>>,
    pub start_at:        Option<DateTime<Utc>>,
    pub all_day:         Option<bool>,
    #[validate(length(min = 7, max = 7))]
    pub color:           Option<String>,
    pub rrule:           Option<String>,
    pub reminders:       Option<Value>,
    pub label_ids:       Option<Vec<Uuid>>,
    pub assignee_ids:    Option<Vec<Uuid>>,
    pub linked_event_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, validator::Validate)]
pub struct UpdateTaskDto {
    pub stack_id:        Option<Uuid>,
    pub parent_task_id:  Option<Uuid>,
    #[validate(length(min = 1, max = 500))]
    pub title:           Option<String>,
    pub description:     Option<String>,
    pub status:          Option<String>,
    #[validate(range(min = 0, max = 9))]
    pub priority:        Option<i16>,
    #[validate(range(min = 0, max = 100))]
    pub percent_complete: Option<i16>,
    pub due_at:          Option<DateTime<Utc>>,
    pub start_at:        Option<DateTime<Utc>>,
    pub all_day:         Option<bool>,
    #[validate(length(min = 7, max = 7))]
    pub color:           Option<String>,
    /// Efface la couleur personnalisée → la tâche hérite de celle du board.
    #[serde(default)]
    pub clear_color:     bool,
    pub rrule:           Option<String>,
    pub reminders:       Option<Value>,
    pub label_ids:       Option<Vec<Uuid>>,
    pub assignee_ids:    Option<Vec<Uuid>>,
    pub linked_event_id: Option<Uuid>,
    /// Permet d'effacer explicitement le lien événement (linked_event_id = null).
    #[serde(default)]
    pub clear_linked_event: bool,
}

#[derive(Debug, Deserialize)]
pub struct MoveTaskDto {
    pub stack_id:   Option<Uuid>,
    pub position:   f64,
    pub sort_order: Option<i32>,
}

/// Déplacement d'une ou plusieurs tâches vers un autre board.
#[derive(Debug, Deserialize, validator::Validate)]
pub struct MoveToBoardDto {
    #[validate(length(min = 1))]
    pub task_ids:        Vec<Uuid>,
    pub target_board_id: Uuid,
    /// Colonne cible ; si absente, la 1ʳᵉ colonne du board cible (ou non classée).
    pub target_stack_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, Default)]
pub struct TasksQuery {
    pub board_id:   Option<Uuid>,
    pub stack_id:   Option<Uuid>,
    pub status:     Option<String>,
    pub collection: Option<String>,
    pub due_before: Option<DateTime<Utc>>,
    pub due_after:  Option<DateTime<Utc>>,
    pub assignee:   Option<Uuid>,
    pub label_id:   Option<Uuid>,
    pub search:     Option<String>,
    /// Inclure les sous-tâches dans la liste (par défaut on ne renvoie que la racine).
    #[serde(default)]
    pub include_subtasks: bool,
}
