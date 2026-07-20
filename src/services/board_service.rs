use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::{Result, TasksError},
    models::board::{Board, BoardShare, CreateBoardDto, ShareBoardDto, UpdateBoardDto},
};

pub struct BoardService;

/// Rang d'une permission (pour comparer read < write < admin).
pub fn perm_rank(p: &str) -> u8 {
    match p {
        "admin" => 3,
        "write" => 2,
        "read"  => 1,
        _       => 0,
    }
}

impl BoardService {
    /// Liste les boards accessibles (propres + partagés). Le board par défaut
    /// (non supprimable, non renommable) est garanti et placé en tête.
    pub async fn list(user_id: Uuid, db: &PgPool) -> Result<Vec<Board>> {
        Self::ensure_default(user_id, db).await?;
        let rows = sqlx::query_as::<_, Board>(
            r#"
            SELECT DISTINCT b.*
            FROM tasks.boards b
            LEFT JOIN tasks.board_shares bs ON bs.board_id = b.id
            WHERE b.owner_id = $1 OR bs.shared_with = $1
            ORDER BY b.is_default DESC, b.is_archived ASC, b.sort_order ASC, b.created_at ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Garantit l'existence du board par défaut de l'utilisateur (le crée sinon).
    pub async fn ensure_default(user_id: Uuid, db: &PgPool) -> Result<()> {
        let exists: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM tasks.boards WHERE owner_id = $1 AND is_default",
        )
        .bind(user_id)
        .fetch_optional(db)
        .await?;
        if exists.is_some() {
            return Ok(());
        }

        let mut tx = db.begin().await?;
        let board = sqlx::query_as::<_, Board>(
            r#"
            INSERT INTO tasks.boards (owner_id, title, board_type, is_default)
            VALUES ($1, 'Tâches', 'kanban', TRUE)
            ON CONFLICT (owner_id) WHERE is_default DO NOTHING
            RETURNING *
            "#,
        )
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(board) = board {
            for (i, title) in ["À faire", "En cours", "Terminé"].iter().enumerate() {
                sqlx::query(
                    "INSERT INTO tasks.stacks (board_id, title, sort_order) VALUES ($1, $2, $3)",
                )
                .bind(board.id)
                .bind(title)
                .bind(i as i32)
                .execute(&mut *tx)
                .await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }

    /// Niveau d'accès de l'utilisateur sur un board : "admin" si propriétaire,
    /// sinon la permission du partage, sinon None.
    pub async fn access_level(board_id: Uuid, user_id: Uuid, db: &PgPool) -> Result<Option<String>> {
        let owner: Option<Uuid> =
            sqlx::query_scalar("SELECT owner_id FROM tasks.boards WHERE id = $1")
                .bind(board_id)
                .fetch_optional(db)
                .await?;
        let owner = match owner {
            Some(o) => o,
            None => return Ok(None),
        };
        if owner == user_id {
            return Ok(Some("admin".to_string()));
        }
        let perm: Option<String> = sqlx::query_scalar(
            "SELECT permission FROM tasks.board_shares WHERE board_id = $1 AND shared_with = $2",
        )
        .bind(board_id)
        .bind(user_id)
        .fetch_optional(db)
        .await?;
        Ok(perm)
    }

    /// Vérifie que l'utilisateur a au moins la permission `min` sur le board.
    /// `min` ∈ {"read","write","admin"}. Sinon NotFound (read) ou Forbidden.
    pub async fn assert_access(board_id: Uuid, user_id: Uuid, min: &str, db: &PgPool) -> Result<()> {
        match Self::access_level(board_id, user_id, db).await? {
            None => Err(TasksError::NotFound(format!("Board {board_id}"))),
            Some(level) => {
                if perm_rank(&level) >= perm_rank(min) {
                    Ok(())
                } else {
                    Err(TasksError::Forbidden)
                }
            }
        }
    }

    pub async fn get(id: Uuid, user_id: Uuid, db: &PgPool) -> Result<Board> {
        Self::assert_access(id, user_id, "read", db).await?;
        let row = sqlx::query_as::<_, Board>("SELECT * FROM tasks.boards WHERE id = $1")
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| TasksError::NotFound(format!("Board {id}")))?;
        Ok(row)
    }

    /// Crée un board ; pour un board kanban, ajoute trois colonnes par défaut.
    pub async fn create(user_id: Uuid, dto: CreateBoardDto, db: &PgPool) -> Result<Board> {
        let color      = dto.color.unwrap_or_else(|| "#1a73e8".to_string());
        let board_type = dto.board_type.unwrap_or_else(|| "kanban".to_string());
        if board_type != "kanban" && board_type != "list" {
            return Err(TasksError::Validation("board_type invalide".to_string()));
        }

        let mut tx = db.begin().await?;

        let board = sqlx::query_as::<_, Board>(
            r#"
            INSERT INTO tasks.boards (id, owner_id, title, description, color, board_type)
            VALUES (COALESCE($6, uuid_generate_v4()), $1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(&dto.title)
        .bind(&dto.description)
        .bind(&color)
        .bind(&board_type)
        .bind(dto.id)
        .fetch_one(&mut *tx)
        .await?;

        if board_type == "kanban" {
            for (i, title) in ["À faire", "En cours", "Terminé"].iter().enumerate() {
                // Client-minted stack ids (sync replay) are honoured in order.
                let sid = dto.initial_stack_ids.as_ref().and_then(|v| v.get(i)).copied();
                sqlx::query(
                    "INSERT INTO tasks.stacks (id, board_id, title, sort_order) VALUES (COALESCE($4, uuid_generate_v4()), $1, $2, $3)",
                )
                .bind(board.id)
                .bind(title)
                .bind(i as i32)
                .bind(sid)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        Ok(board)
    }

    pub async fn update(id: Uuid, user_id: Uuid, dto: UpdateBoardDto, db: &PgPool) -> Result<Board> {
        Self::assert_access(id, user_id, "admin", db).await?;
        let current = sqlx::query_as::<_, Board>("SELECT * FROM tasks.boards WHERE id = $1")
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| TasksError::NotFound(format!("Board {id}")))?;

        // Le board par défaut ne peut être ni renommé ni archivé (toujours visible).
        let title       = if current.is_default { current.title.clone() } else { dto.title.unwrap_or(current.title) };
        let description = dto.description.or(current.description);
        let color       = dto.color.unwrap_or(current.color);
        let board_type  = dto.board_type.unwrap_or(current.board_type);
        let is_archived = if current.is_default { false } else { dto.is_archived.unwrap_or(current.is_archived) };
        let sort_order  = dto.sort_order.unwrap_or(current.sort_order);

        let row = sqlx::query_as::<_, Board>(
            r#"
            UPDATE tasks.boards
            SET title = $2, description = $3, color = $4, board_type = $5,
                is_archived = $6, sort_order = $7
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&title)
        .bind(&description)
        .bind(&color)
        .bind(&board_type)
        .bind(is_archived)
        .bind(sort_order)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    pub async fn delete(id: Uuid, user_id: Uuid, db: &PgPool) -> Result<()> {
        Self::assert_access(id, user_id, "admin", db).await?;
        let is_default: bool =
            sqlx::query_scalar("SELECT is_default FROM tasks.boards WHERE id = $1")
                .bind(id)
                .fetch_optional(db)
                .await?
                .unwrap_or(false);
        if is_default {
            return Err(TasksError::Conflict(
                "Le board par défaut ne peut pas être supprimé".to_string(),
            ));
        }
        sqlx::query("DELETE FROM tasks.boards WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;
        Ok(())
    }

    pub async fn share(id: Uuid, user_id: Uuid, dto: ShareBoardDto, db: &PgPool) -> Result<BoardShare> {
        Self::assert_access(id, user_id, "admin", db).await?;
        // Le board par défaut n'est jamais partagé (ses tâches peuvent l'être via l'attribution).
        let is_default: bool =
            sqlx::query_scalar("SELECT is_default FROM tasks.boards WHERE id = $1")
                .bind(id)
                .fetch_optional(db)
                .await?
                .unwrap_or(false);
        if is_default {
            return Err(TasksError::Conflict(
                "Le board par défaut ne peut pas être partagé".to_string(),
            ));
        }
        let permission = dto.permission.unwrap_or_else(|| "read".to_string());
        if !["read", "write", "admin"].contains(&permission.as_str()) {
            return Err(TasksError::Validation("permission invalide".to_string()));
        }
        let row = sqlx::query_as::<_, BoardShare>(
            r#"
            INSERT INTO tasks.board_shares (board_id, shared_with, permission)
            VALUES ($1, $2, $3)
            ON CONFLICT (board_id, shared_with)
            DO UPDATE SET permission = EXCLUDED.permission
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(dto.user_id)
        .bind(&permission)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    pub async fn unshare(id: Uuid, user_id: Uuid, shared_with: Uuid, db: &PgPool) -> Result<()> {
        Self::assert_access(id, user_id, "admin", db).await?;
        sqlx::query("DELETE FROM tasks.board_shares WHERE board_id = $1 AND shared_with = $2")
            .bind(id)
            .bind(shared_with)
            .execute(db)
            .await?;
        Ok(())
    }

    pub async fn list_shares(id: Uuid, user_id: Uuid, db: &PgPool) -> Result<Vec<BoardShare>> {
        Self::assert_access(id, user_id, "read", db).await?;
        let rows = sqlx::query_as::<_, BoardShare>(
            "SELECT * FROM tasks.board_shares WHERE board_id = $1 ORDER BY created_at",
        )
        .bind(id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }
}
