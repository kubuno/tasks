use kubuno_db::{params, DbPool};
use uuid::Uuid;

use crate::{
    config::{InstanceConfig, BYTES_PER_MB},
    errors::{Result, TasksError},
    models::attachment::{Attachment, CreateAttachmentDto},
    services::{board_service::BoardService, task_service::TaskService},
};

pub struct AttachmentService;

impl AttachmentService {
    pub async fn list(task_id: Uuid, user_id: Uuid, db: &DbPool) -> Result<Vec<Attachment>> {
        let board_id = TaskService::board_of_task(task_id, db).await?;
        BoardService::assert_access(board_id, user_id, "read", db).await?;
        let rows = db
            .fetch_all_as::<Attachment>(
                "SELECT * FROM tasks.attachments WHERE task_id = $1 ORDER BY created_at",
                params![task_id],
            )
            .await?;
        Ok(rows)
    }

    pub async fn create(
        task_id: Uuid,
        user_id: Uuid,
        dto: CreateAttachmentDto,
        instance: InstanceConfig,
        db: &DbPool,
    ) -> Result<Attachment> {
        // Enforce the admin's declared-size ceiling before touching the database.
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

        // Attachments are carried by neither delta feed, so no change_seq is bumped.
        let id = kubuno_db::new_id();
        db.execute(
            "INSERT INTO tasks.attachments (id, task_id, file_id, filename, mime_type, size_bytes)
             VALUES ($1, $2, $3, $4, $5, $6)",
            params![id, task_id, dto.file_id, dto.filename, dto.mime_type, dto.size_bytes],
        )
        .await?;

        db.fetch_one_as::<Attachment>("SELECT * FROM tasks.attachments WHERE id = $1", params![id])
            .await
            .map_err(Into::into)
    }

    pub async fn delete(id: Uuid, user_id: Uuid, db: &DbPool) -> Result<()> {
        let task_id: Option<Uuid> = db
            .fetch_optional_scalar("SELECT task_id FROM tasks.attachments WHERE id = $1", params![id])
            .await?;
        if let Some(task_id) = task_id {
            let board_id = TaskService::board_of_task(task_id, db).await?;
            BoardService::assert_access(board_id, user_id, "write", db).await?;
            db.execute("DELETE FROM tasks.attachments WHERE id = $1", params![id]).await?;
        }
        Ok(())
    }
}
