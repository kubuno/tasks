use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{Result, TasksError},
    models::comment::{BoardComment, CreateCommentDto, UpdateCommentDto},
    services::board_service::BoardService,
};

pub struct BoardCommentService;

impl BoardCommentService {
    pub async fn list(board_id: Uuid, user_id: Uuid, db: &PgPool) -> Result<Vec<BoardComment>> {
        BoardService::assert_access(board_id, user_id, "read", db).await?;
        let rows = sqlx::query_as::<_, BoardComment>(
            "SELECT * FROM tasks.board_comments WHERE board_id = $1 ORDER BY created_at",
        )
        .bind(board_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Tout utilisateur ayant accès au board (propriétaire ou partagé) peut commenter.
    pub async fn create(board_id: Uuid, user_id: Uuid, dto: CreateCommentDto, db: &PgPool) -> Result<BoardComment> {
        BoardService::assert_access(board_id, user_id, "read", db).await?;
        let row = sqlx::query_as::<_, BoardComment>(
            "INSERT INTO tasks.board_comments (board_id, author_id, body) VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(board_id)
        .bind(user_id)
        .bind(&dto.body)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    /// Seul l'auteur peut modifier son commentaire.
    pub async fn update(id: Uuid, user_id: Uuid, dto: UpdateCommentDto, db: &PgPool) -> Result<BoardComment> {
        let author: Option<Uuid> =
            sqlx::query_scalar("SELECT author_id FROM tasks.board_comments WHERE id = $1")
                .bind(id)
                .fetch_optional(db)
                .await?;
        match author {
            None => Err(TasksError::NotFound(format!("Comment {id}"))),
            Some(a) if a != user_id => Err(TasksError::Forbidden),
            Some(_) => {
                let row = sqlx::query_as::<_, BoardComment>(
                    "UPDATE tasks.board_comments SET body = $2 WHERE id = $1 RETURNING *",
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
            "SELECT author_id, board_id FROM tasks.board_comments WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(db)
        .await?;
        let (author, board_id) = row.ok_or_else(|| TasksError::NotFound(format!("Comment {id}")))?;
        if author != user_id {
            BoardService::assert_access(board_id, user_id, "admin", db).await?;
        }
        sqlx::query("DELETE FROM tasks.board_comments WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;
        Ok(())
    }
}
