use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{Result, TasksError},
    models::stack::{CreateStackDto, Stack, UpdateStackDto},
    services::board_service::BoardService,
};

pub struct StackService;

impl StackService {
    pub async fn list(board_id: Uuid, user_id: Uuid, db: &PgPool) -> Result<Vec<Stack>> {
        BoardService::assert_access(board_id, user_id, "read", db).await?;
        let rows = sqlx::query_as::<_, Stack>(
            "SELECT * FROM tasks.stacks WHERE board_id = $1 ORDER BY sort_order, created_at",
        )
        .bind(board_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn create(board_id: Uuid, user_id: Uuid, dto: CreateStackDto, db: &PgPool) -> Result<Stack> {
        BoardService::assert_access(board_id, user_id, "write", db).await?;
        let sort_order = match dto.sort_order {
            Some(s) => s,
            None => sqlx::query_scalar::<_, Option<i32>>(
                "SELECT MAX(sort_order) FROM tasks.stacks WHERE board_id = $1",
            )
            .bind(board_id)
            .fetch_one(db)
            .await?
            .map(|m| m + 1)
            .unwrap_or(0),
        };
        let row = sqlx::query_as::<_, Stack>(
            "INSERT INTO tasks.stacks (board_id, title, sort_order) VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(board_id)
        .bind(&dto.title)
        .bind(sort_order)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    async fn board_of(stack_id: Uuid, db: &PgPool) -> Result<Uuid> {
        sqlx::query_scalar::<_, Uuid>("SELECT board_id FROM tasks.stacks WHERE id = $1")
            .bind(stack_id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| TasksError::NotFound(format!("Stack {stack_id}")))
    }

    pub async fn update(id: Uuid, user_id: Uuid, dto: UpdateStackDto, db: &PgPool) -> Result<Stack> {
        let board_id = Self::board_of(id, db).await?;
        BoardService::assert_access(board_id, user_id, "write", db).await?;
        let current = sqlx::query_as::<_, Stack>("SELECT * FROM tasks.stacks WHERE id = $1")
            .bind(id)
            .fetch_one(db)
            .await?;
        let title      = dto.title.unwrap_or(current.title);
        let sort_order = dto.sort_order.unwrap_or(current.sort_order);
        let row = sqlx::query_as::<_, Stack>(
            "UPDATE tasks.stacks SET title = $2, sort_order = $3 WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(&title)
        .bind(sort_order)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    pub async fn delete(id: Uuid, user_id: Uuid, db: &PgPool) -> Result<()> {
        let board_id = Self::board_of(id, db).await?;
        BoardService::assert_access(board_id, user_id, "write", db).await?;
        sqlx::query("DELETE FROM tasks.stacks WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;
        Ok(())
    }

    /// Réordonne les colonnes d'un board selon la liste d'IDs fournie.
    pub async fn reorder(board_id: Uuid, user_id: Uuid, ordered_ids: Vec<Uuid>, db: &PgPool) -> Result<Vec<Stack>> {
        BoardService::assert_access(board_id, user_id, "write", db).await?;
        let mut tx = db.begin().await?;
        for (i, sid) in ordered_ids.iter().enumerate() {
            sqlx::query(
                "UPDATE tasks.stacks SET sort_order = $1 WHERE id = $2 AND board_id = $3",
            )
            .bind(i as i32)
            .bind(sid)
            .bind(board_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Self::list(board_id, user_id, db).await
    }
}
