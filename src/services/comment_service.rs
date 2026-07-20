use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{Result, TasksError},
    models::comment::{Comment, CreateCommentDto, UpdateCommentDto},
    services::{board_service::BoardService, task_service::TaskService},
};

pub struct CommentService;

impl CommentService {
    pub async fn list(task_id: Uuid, user_id: Uuid, db: &PgPool) -> Result<Vec<Comment>> {
        TaskService::assert_task_access(task_id, user_id, "read", db).await?;
        let rows = sqlx::query_as::<_, Comment>(
            "SELECT * FROM tasks.comments WHERE task_id = $1 ORDER BY created_at",
        )
        .bind(task_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn create(task_id: Uuid, user_id: Uuid, dto: CreateCommentDto, db: &PgPool) -> Result<Comment> {
        // Tout utilisateur ayant accès à la tâche (board partagé OU assigné) peut commenter.
        TaskService::assert_task_access(task_id, user_id, "read", db).await?;
        let row = sqlx::query_as::<_, Comment>(
            "INSERT INTO tasks.comments (id, task_id, author_id, body) VALUES (COALESCE($4, uuid_generate_v4()), $1, $2, $3) RETURNING *",
        )
        .bind(task_id)
        .bind(user_id)
        .bind(&dto.body)
        .bind(dto.id)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    pub async fn update(id: Uuid, user_id: Uuid, dto: UpdateCommentDto, db: &PgPool) -> Result<Comment> {
        let author: Option<Uuid> =
            sqlx::query_scalar("SELECT author_id FROM tasks.comments WHERE id = $1")
                .bind(id)
                .fetch_optional(db)
                .await?;
        match author {
            None => Err(TasksError::NotFound(format!("Comment {id}"))),
            Some(a) if a != user_id => Err(TasksError::Forbidden),
            Some(_) => {
                let row = sqlx::query_as::<_, Comment>(
                    "UPDATE tasks.comments SET body = $2 WHERE id = $1 RETURNING *",
                )
                .bind(id)
                .bind(&dto.body)
                .fetch_one(db)
                .await?;
                Ok(row)
            }
        }
    }

    pub async fn delete(id: Uuid, user_id: Uuid, db: &PgPool) -> Result<()> {
        let row: Option<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT author_id, task_id FROM tasks.comments WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(db)
        .await?;
        let (author, task_id) = row.ok_or_else(|| TasksError::NotFound(format!("Comment {id}")))?;
        if author != user_id {
            // Les admins du board peuvent aussi supprimer.
            let board_id = TaskService::board_of_task(task_id, db).await?;
            BoardService::assert_access(board_id, user_id, "admin", db).await?;
        }
        sqlx::query("DELETE FROM tasks.comments WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;
        Ok(())
    }
}
