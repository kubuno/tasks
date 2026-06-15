use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{Result, TasksError},
    models::label::{CreateLabelDto, Label, UpdateLabelDto},
    services::board_service::BoardService,
};

pub struct LabelService;

impl LabelService {
    pub async fn list(board_id: Uuid, user_id: Uuid, db: &PgPool) -> Result<Vec<Label>> {
        BoardService::assert_access(board_id, user_id, "read", db).await?;
        let rows = sqlx::query_as::<_, Label>(
            "SELECT * FROM tasks.labels WHERE board_id = $1 ORDER BY title",
        )
        .bind(board_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn create(board_id: Uuid, user_id: Uuid, dto: CreateLabelDto, db: &PgPool) -> Result<Label> {
        BoardService::assert_access(board_id, user_id, "write", db).await?;
        let color = dto.color.unwrap_or_else(|| "#888888".to_string());
        let row = sqlx::query_as::<_, Label>(
            r#"
            INSERT INTO tasks.labels (board_id, title, color)
            VALUES ($1, $2, $3)
            ON CONFLICT (board_id, title) DO UPDATE SET color = EXCLUDED.color
            RETURNING *
            "#,
        )
        .bind(board_id)
        .bind(&dto.title)
        .bind(&color)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    async fn board_of(label_id: Uuid, db: &PgPool) -> Result<Uuid> {
        sqlx::query_scalar::<_, Uuid>("SELECT board_id FROM tasks.labels WHERE id = $1")
            .bind(label_id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| TasksError::NotFound(format!("Label {label_id}")))
    }

    pub async fn update(id: Uuid, user_id: Uuid, dto: UpdateLabelDto, db: &PgPool) -> Result<Label> {
        let board_id = Self::board_of(id, db).await?;
        BoardService::assert_access(board_id, user_id, "write", db).await?;
        let current = sqlx::query_as::<_, Label>("SELECT * FROM tasks.labels WHERE id = $1")
            .bind(id)
            .fetch_one(db)
            .await?;
        let title = dto.title.unwrap_or(current.title);
        let color = dto.color.unwrap_or(current.color);
        let row = sqlx::query_as::<_, Label>(
            "UPDATE tasks.labels SET title = $2, color = $3 WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(&title)
        .bind(&color)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    pub async fn delete(id: Uuid, user_id: Uuid, db: &PgPool) -> Result<()> {
        let board_id = Self::board_of(id, db).await?;
        BoardService::assert_access(board_id, user_id, "write", db).await?;
        sqlx::query("DELETE FROM tasks.labels WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;
        Ok(())
    }
}
