use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    config::{InstanceConfig, BYTES_PER_MB},
    errors::{Result, TasksError},
    models::attachment::{Attachment, CreateAttachmentDto},
    services::{board_service::BoardService, task_service::TaskService},
};

pub struct AttachmentService;

impl AttachmentService {
    pub async fn list(task_id: Uuid, user_id: Uuid, db: &PgPool) -> Result<Vec<Attachment>> {
        let board_id = TaskService::board_of_task(task_id, db).await?;
        BoardService::assert_access(board_id, user_id, "read", db).await?;
        let rows = sqlx::query_as::<_, Attachment>(
            "SELECT * FROM tasks.attachments WHERE task_id = $1 ORDER BY created_at",
        )
        .bind(task_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn create(
        task_id: Uuid,
        user_id: Uuid,
        dto: CreateAttachmentDto,
        instance: InstanceConfig,
        db: &PgPool,
    ) -> Result<Attachment> {
        // Enforce the admin's declared-size ceiling before touching the database.
        // The real bytes live in drive/core (referenced by `file_id`); this caps
        // the size a task attachment declares at record time. `0` = no ceiling.
        if instance.attachment_max_mb > 0 {
            let max_bytes = instance.attachment_max_mb * BYTES_PER_MB;
            if dto.size_bytes.unwrap_or(0) > max_bytes {
                return Err(TasksError::PayloadTooLarge(format!(
                    "pièce jointe trop volumineuse (max {} Mo)",
                    instance.attachment_max_mb
                )));
            }
        }
        let board_id = TaskService::board_of_task(task_id, db).await?;
        BoardService::assert_access(board_id, user_id, "write", db).await?;
        let row = sqlx::query_as::<_, Attachment>(
            r#"
            INSERT INTO tasks.attachments (task_id, file_id, filename, mime_type, size_bytes)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(task_id)
        .bind(dto.file_id)
        .bind(&dto.filename)
        .bind(&dto.mime_type)
        .bind(dto.size_bytes)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    pub async fn delete(id: Uuid, user_id: Uuid, db: &PgPool) -> Result<()> {
        let task_id: Option<Uuid> =
            sqlx::query_scalar("SELECT task_id FROM tasks.attachments WHERE id = $1")
                .bind(id)
                .fetch_optional(db)
                .await?;
        if let Some(task_id) = task_id {
            let board_id = TaskService::board_of_task(task_id, db).await?;
            BoardService::assert_access(board_id, user_id, "write", db).await?;
            sqlx::query("DELETE FROM tasks.attachments WHERE id = $1")
                .bind(id)
                .execute(db)
                .await?;
        }
        Ok(())
    }
}
