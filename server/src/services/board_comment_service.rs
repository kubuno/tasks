use kubuno_db::{params, DbPool};
use uuid::Uuid;

use crate::{
    errors::{Result, TasksError},
    models::comment::{BoardComment, CreateCommentDto, UpdateCommentDto},
    services::board_service::BoardService,
    sync,
};

pub struct BoardCommentService;

impl BoardCommentService {
    pub async fn list(board_id: Uuid, user_id: Uuid, db: &DbPool) -> Result<Vec<BoardComment>> {
        BoardService::assert_access(board_id, user_id, "read", db).await?;
        let rows = db
            .fetch_all_as::<BoardComment>(
                "SELECT * FROM tasks.board_comments WHERE board_id = $1 ORDER BY created_at",
                params![board_id],
            )
            .await?;
        Ok(rows)
    }

    /// Tout utilisateur ayant accès au board (propriétaire ou partagé) peut commenter.
    pub async fn create(board_id: Uuid, user_id: Uuid, dto: CreateCommentDto, db: &DbPool) -> Result<BoardComment> {
        BoardService::assert_access(board_id, user_id, "read", db).await?;
        let comment_id = dto.id.unwrap_or_else(kubuno_db::new_id);

        let mut tx = db.begin().await?;
        tx.execute(
            "INSERT INTO tasks.board_comments (id, board_id, author_id, body) VALUES ($1, $2, $3, $4)",
            params![comment_id, board_id, user_id, dto.body],
        )
        .await?;
        sync::touch_board(&mut tx, board_id).await?;
        tx.commit().await?;

        db.fetch_one_as::<BoardComment>(
            "SELECT * FROM tasks.board_comments WHERE id = $1",
            params![comment_id],
        )
        .await
        .map_err(Into::into)
    }

    /// Seul l'auteur peut modifier son commentaire.
    pub async fn update(id: Uuid, user_id: Uuid, dto: UpdateCommentDto, db: &DbPool) -> Result<BoardComment> {
        let row: Option<(Uuid, Uuid)> = db
            .fetch_optional_as("SELECT author_id, board_id FROM tasks.board_comments WHERE id = $1", params![id])
            .await?;
        let (author, board_id) = row.ok_or_else(|| TasksError::NotFound(format!("Comment {id}")))?;
        if author != user_id {
            return Err(TasksError::Forbidden);
        }
        let mut tx = db.begin().await?;
        tx.execute("UPDATE tasks.board_comments SET body = $1 WHERE id = $2", params![dto.body, id])
            .await?;
        sync::touch_board(&mut tx, board_id).await?;
        tx.commit().await?;

        db.fetch_one_as::<BoardComment>("SELECT * FROM tasks.board_comments WHERE id = $1", params![id])
            .await
            .map_err(Into::into)
    }

    pub async fn delete(id: Uuid, user_id: Uuid, db: &DbPool) -> Result<()> {
        let row: Option<(Uuid, Uuid)> = db
            .fetch_optional_as("SELECT author_id, board_id FROM tasks.board_comments WHERE id = $1", params![id])
            .await?;
        let (author, board_id) = row.ok_or_else(|| TasksError::NotFound(format!("Comment {id}")))?;
        if author != user_id {
            BoardService::assert_access(board_id, user_id, "admin", db).await?;
        }
        let mut tx = db.begin().await?;
        tx.execute("DELETE FROM tasks.board_comments WHERE id = $1", params![id]).await?;
        sync::touch_board(&mut tx, board_id).await?;
        tx.commit().await?;
        Ok(())
    }
}
