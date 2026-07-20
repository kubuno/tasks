use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Stack {
    pub id:         Uuid,
    pub board_id:   Uuid,
    pub title:      String,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, validator::Validate)]
pub struct CreateStackDto {
    /// Optional client-minted id (local-first sync replay) — honoured verbatim.
    #[serde(default)]
    pub id: Option<Uuid>,
    #[validate(length(min = 1, max = 255))]
    pub title:      String,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStackDto {
    pub title:      Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ReorderStacksDto {
    pub ordered_ids: Vec<Uuid>,
}
