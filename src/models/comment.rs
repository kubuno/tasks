use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Comment {
    pub id:         Uuid,
    pub task_id:    Uuid,
    pub author_id:  Uuid,
    pub body:       String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, validator::Validate)]
pub struct CreateCommentDto {
    #[validate(length(min = 1, max = 10000))]
    pub body: String,
}

#[derive(Debug, Deserialize, validator::Validate)]
pub struct UpdateCommentDto {
    #[validate(length(min = 1, max = 10000))]
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BoardComment {
    pub id:         Uuid,
    pub board_id:   Uuid,
    pub author_id:  Uuid,
    pub body:       String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
