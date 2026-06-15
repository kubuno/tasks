use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Label {
    pub id:         Uuid,
    pub board_id:   Uuid,
    pub title:      String,
    pub color:      String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, validator::Validate)]
pub struct CreateLabelDto {
    #[validate(length(min = 1, max = 100))]
    pub title: String,
    #[validate(length(min = 7, max = 7))]
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLabelDto {
    pub title: Option<String>,
    pub color: Option<String>,
}
