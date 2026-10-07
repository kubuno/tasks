use kubuno_db::{params, DbPool};
use uuid::Uuid;

use crate::{
    errors::{Result, TasksError},
    models::stack::{CreateStackDto, Stack, UpdateStackDto},
    services::board_service::BoardService,
    sync,
};

pub struct StackService;

impl StackService {
    pub async fn list(board_id: Uuid, user_id: Uuid, db: &DbPool) -> Result<Vec<Stack>> {
        BoardService::assert_access(board_id, user_id, "read", db).await?;
        let rows = db
            .fetch_all_as::<Stack>(
                "SELECT * FROM tasks.stacks WHERE board_id = $1 ORDER BY sort_order, created_at",
                params![board_id],
            )
            .await?;
        Ok(rows)
    }

    pub async fn create(board_id: Uuid, user_id: Uuid, dto: CreateStackDto, db: &DbPool) -> Result<Stack> {
        BoardService::assert_access(board_id, user_id, "write", db).await?;
        let sort_order = match dto.sort_order {
            Some(s) => s,
            None => db
                .fetch_scalar::<Option<i32>>(
                    "SELECT MAX(sort_order) FROM tasks.stacks WHERE board_id = $1",
                    params![board_id],
                )
                .await?
                .map(|m| m + 1)
                .unwrap_or(0),
        };
        let stack_id = dto.id.unwrap_or_else(kubuno_db::new_id);

        let mut tx = db.begin().await?;
        tx.execute(
            "INSERT INTO tasks.stacks (id, board_id, title, sort_order) VALUES ($1, $2, $3, $4)",
            params![stack_id, board_id, dto.title, sort_order],
        )
        .await?;
        sync::touch_board(&mut tx, board_id).await?;
        tx.commit().await?;

        db.fetch_one_as::<Stack>("SELECT * FROM tasks.stacks WHERE id = $1", params![stack_id])
            .await
            .map_err(Into::into)
    }

    async fn board_of(stack_id: Uuid, db: &DbPool) -> Result<Uuid> {
        db.fetch_optional_scalar::<Uuid>(
            "SELECT board_id FROM tasks.stacks WHERE id = $1",
            params![stack_id],
        )
        .await?
        .ok_or_else(|| TasksError::NotFound(format!("Stack {stack_id}")))
    }

    pub async fn update(id: Uuid, user_id: Uuid, dto: UpdateStackDto, db: &DbPool) -> Result<Stack> {
        let board_id = Self::board_of(id, db).await?;
        BoardService::assert_access(board_id, user_id, "write", db).await?;
        let current = db
            .fetch_one_as::<Stack>("SELECT * FROM tasks.stacks WHERE id = $1", params![id])
            .await?;
        let title      = dto.title.unwrap_or(current.title);
        let sort_order = dto.sort_order.unwrap_or(current.sort_order);

        let mut tx = db.begin().await?;
        tx.execute(
            "UPDATE tasks.stacks SET title = $1, sort_order = $2 WHERE id = $3",
            params![title, sort_order, id],
        )
        .await?;
        sync::touch_board(&mut tx, board_id).await?;
        tx.commit().await?;

        db.fetch_one_as::<Stack>("SELECT * FROM tasks.stacks WHERE id = $1", params![id])
            .await
            .map_err(Into::into)
    }

    pub async fn delete(id: Uuid, user_id: Uuid, db: &DbPool) -> Result<()> {
        let board_id = Self::board_of(id, db).await?;
        BoardService::assert_access(board_id, user_id, "write", db).await?;
        let mut tx = db.begin().await?;
        tx.execute("DELETE FROM tasks.stacks WHERE id = $1", params![id]).await?;
        sync::touch_board(&mut tx, board_id).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Réordonne les colonnes d'un board selon la liste d'IDs fournie.
    pub async fn reorder(board_id: Uuid, user_id: Uuid, ordered_ids: Vec<Uuid>, db: &DbPool) -> Result<Vec<Stack>> {
        BoardService::assert_access(board_id, user_id, "write", db).await?;
        let mut tx = db.begin().await?;
        for (i, sid) in ordered_ids.iter().enumerate() {
            tx.execute(
                "UPDATE tasks.stacks SET sort_order = $1 WHERE id = $2 AND board_id = $3",
                params![i as i32, sid, board_id],
            )
            .await?;
        }
        sync::touch_board(&mut tx, board_id).await?;
        tx.commit().await?;
        Self::list(board_id, user_id, db).await
    }
}
