use kubuno_db::dialect::Assign;
use kubuno_db::{params, DbPool};
use uuid::Uuid;

use crate::{
    errors::{Result, TasksError},
    models::label::{CreateLabelDto, Label, UpdateLabelDto},
    services::board_service::BoardService,
    sync,
};

pub struct LabelService;

impl LabelService {
    pub async fn list(board_id: Uuid, user_id: Uuid, db: &DbPool) -> Result<Vec<Label>> {
        BoardService::assert_access(board_id, user_id, "read", db).await?;
        let rows = db
            .fetch_all_as::<Label>(
                "SELECT * FROM tasks.labels WHERE board_id = $1 ORDER BY title",
                params![board_id],
            )
            .await?;
        Ok(rows)
    }

    pub async fn create(board_id: Uuid, user_id: Uuid, dto: CreateLabelDto, db: &DbPool) -> Result<Label> {
        BoardService::assert_access(board_id, user_id, "write", db).await?;
        let color = dto.color.unwrap_or_else(|| "#888888".to_string());
        let label_id = dto.id.unwrap_or_else(kubuno_db::new_id);
        let clause = db.backend().upsert("labels", &["board_id", "title"], &[Assign::Incoming("color")]);

        let mut tx = db.begin().await?;
        tx.execute(
            &format!(
                "INSERT INTO tasks.labels (id, board_id, title, color) VALUES ($1, $2, $3, $4){clause}"
            ),
            params![label_id, board_id, &dto.title, color],
        )
        .await?;
        sync::touch_board(&mut tx, board_id).await?;
        tx.commit().await?;

        // Re-read by the natural key: on a conflict the row keeps its original id,
        // so the (board_id, title) unique key is what finds it on every engine.
        db.fetch_one_as::<Label>(
            "SELECT * FROM tasks.labels WHERE board_id = $1 AND title = $2",
            params![board_id, &dto.title],
        )
        .await
        .map_err(Into::into)
    }

    async fn board_of(label_id: Uuid, db: &DbPool) -> Result<Uuid> {
        db.fetch_optional_scalar::<Uuid>(
            "SELECT board_id FROM tasks.labels WHERE id = $1",
            params![label_id],
        )
        .await?
        .ok_or_else(|| TasksError::NotFound(format!("Label {label_id}")))
    }

    pub async fn update(id: Uuid, user_id: Uuid, dto: UpdateLabelDto, db: &DbPool) -> Result<Label> {
        let board_id = Self::board_of(id, db).await?;
        BoardService::assert_access(board_id, user_id, "write", db).await?;
        let current = db
            .fetch_one_as::<Label>("SELECT * FROM tasks.labels WHERE id = $1", params![id])
            .await?;
        let title = dto.title.unwrap_or(current.title);
        let color = dto.color.unwrap_or(current.color);

        let mut tx = db.begin().await?;
        tx.execute(
            "UPDATE tasks.labels SET title = $2, color = $3 WHERE id = $1",
            params![id, title, color],
        )
        .await?;
        sync::touch_board(&mut tx, board_id).await?;
        tx.commit().await?;

        db.fetch_one_as::<Label>("SELECT * FROM tasks.labels WHERE id = $1", params![id])
            .await
            .map_err(Into::into)
    }

    pub async fn delete(id: Uuid, user_id: Uuid, db: &DbPool) -> Result<()> {
        let board_id = Self::board_of(id, db).await?;
        BoardService::assert_access(board_id, user_id, "write", db).await?;
        let mut tx = db.begin().await?;
        tx.execute("DELETE FROM tasks.labels WHERE id = $1", params![id]).await?;
        sync::touch_board(&mut tx, board_id).await?;
        tx.commit().await?;
        Ok(())
    }
}
