use kubuno_db::{params, DbPool};
use uuid::Uuid;

use crate::{
    errors::{Result, TasksError},
    models::comment::{Comment, CreateCommentDto, UpdateCommentDto},
    services::{board_service::BoardService, task_service::TaskService},
    sync,
};

pub struct CommentService;

impl CommentService {
    pub async fn list(task_id: Uuid, user_id: Uuid, db: &DbPool) -> Result<Vec<Comment>> {
        TaskService::assert_task_access(task_id, user_id, "read", db).await?;
        let rows = db
            .fetch_all_as::<Comment>(
                "SELECT * FROM tasks.comments WHERE task_id = $1 ORDER BY created_at",
                params![task_id],
            )
            .await?;
        Ok(rows)
    }

    pub async fn create(task_id: Uuid, user_id: Uuid, dto: CreateCommentDto, db: &DbPool) -> Result<Comment> {
        // Tout utilisateur ayant accès à la tâche (board partagé OU assigné) peut commenter.
        TaskService::assert_task_access(task_id, user_id, "read", db).await?;
        let comment_id = dto.id.unwrap_or_else(kubuno_db::new_id);

        let mut tx = db.begin().await?;
        tx.execute(
            "INSERT INTO tasks.comments (id, task_id, author_id, body) VALUES ($1, $2, $3, $4)",
            params![comment_id, task_id, user_id, dto.body],
        )
        .await?;
        sync::touch_task(&mut tx, task_id).await?;
        tx.commit().await?;

        db.fetch_one_as::<Comment>("SELECT * FROM tasks.comments WHERE id = $1", params![comment_id])
            .await
            .map_err(Into::into)
    }

    pub async fn update(id: Uuid, user_id: Uuid, dto: UpdateCommentDto, db: &DbPool) -> Result<Comment> {
        let row: Option<(Uuid, Uuid)> = db
            .fetch_optional_as("SELECT author_id, task_id FROM tasks.comments WHERE id = $1", params![id])
            .await?;
        let (author, task_id) = row.ok_or_else(|| TasksError::NotFound(format!("Comment {id}")))?;
        if author != user_id {
            return Err(TasksError::Forbidden);
        }
        let mut tx = db.begin().await?;
        tx.execute("UPDATE tasks.comments SET body = $1 WHERE id = $2", params![dto.body, id])
            .await?;
        sync::touch_task(&mut tx, task_id).await?;
        tx.commit().await?;

        db.fetch_one_as::<Comment>("SELECT * FROM tasks.comments WHERE id = $1", params![id])
            .await
            .map_err(Into::into)
    }

    pub async fn delete(id: Uuid, user_id: Uuid, db: &DbPool) -> Result<()> {
        let row: Option<(Uuid, Uuid)> = db
            .fetch_optional_as("SELECT author_id, task_id FROM tasks.comments WHERE id = $1", params![id])
            .await?;
        let (author, task_id) = row.ok_or_else(|| TasksError::NotFound(format!("Comment {id}")))?;
        if author != user_id {
            // Les admins du board peuvent aussi supprimer.
            let board_id = TaskService::board_of_task(task_id, db).await?;
            BoardService::assert_access(board_id, user_id, "admin", db).await?;
        }
        let mut tx = db.begin().await?;
        tx.execute("DELETE FROM tasks.comments WHERE id = $1", params![id]).await?;
        sync::touch_task(&mut tx, task_id).await?;
        tx.commit().await?;
        Ok(())
    }
}
