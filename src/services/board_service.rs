use kubuno_db::dialect::Assign;
use kubuno_db::{params, DbPool};
use uuid::Uuid;

use crate::{
    config::InstanceConfig,
    errors::{Result, TasksError},
    models::board::{Board, BoardShare, CreateBoardDto, ShareBoardDto, UpdateBoardDto},
    sync,
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

/// Whether a sqlx error is a UNIQUE / primary-key violation, portably across the
/// three engines. Used to make `ensure_default` idempotent under a concurrent
/// first load (PostgreSQL and SQLite enforce the partial unique index; MySQL has
/// no partial index and so never raises here).
fn is_unique_violation(e: &sqlx::Error) -> bool {
    e.as_database_error().map(|d| d.is_unique_violation()).unwrap_or(false)
}

impl BoardService {
    /// Liste les boards accessibles (propres + partagés). Le board par défaut
    /// (non supprimable, non renommable) est garanti et placé en tête.
    pub async fn list(user_id: Uuid, db: &DbPool) -> Result<Vec<Board>> {
        Self::ensure_default(user_id, db).await?;
        let rows = db
            .fetch_all_as::<Board>(
                r#"
                SELECT DISTINCT b.*
                FROM tasks.boards b
                LEFT JOIN tasks.board_shares bs ON bs.board_id = b.id
                WHERE b.owner_id = $1 OR bs.shared_with = $2
                ORDER BY b.is_default DESC, b.is_archived ASC, b.sort_order ASC, b.created_at ASC
                "#,
                params![user_id, user_id],
            )
            .await?;
        Ok(rows)
    }

    /// Garantit l'existence du board par défaut de l'utilisateur (le crée sinon).
    pub async fn ensure_default(user_id: Uuid, db: &DbPool) -> Result<()> {
        let exists: Option<Uuid> = db
            .fetch_optional_scalar(
                "SELECT id FROM tasks.boards WHERE owner_id = $1 AND is_default",
                params![user_id],
            )
            .await?;
        if exists.is_some() {
            return Ok(());
        }

        let board_id = kubuno_db::new_id();
        let mut tx = db.begin().await?;
        let seq = sync::next_board_seq(&mut tx).await?;
        // The insert may lose a race with a concurrent first load (PostgreSQL /
        // SQLite enforce the "one default per owner" partial unique index). Treat
        // that as success: the board exists, which is all the caller wanted.
        match tx
            .execute(
                "INSERT INTO tasks.boards
                   (id, owner_id, title, board_type, is_default, caldav_token, ctag, change_seq)
                 VALUES ($1, $2, 'Tâches', 'kanban', $3, $4, $5, $6)",
                params![board_id, user_id, true, sync::new_tag(), sync::new_tag(), seq],
            )
            .await
        {
            Ok(_) => {}
            Err(e) if is_unique_violation(&e) => {
                let _ = tx.rollback().await;
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        }

        for (i, title) in ["À faire", "En cours", "Terminé"].iter().enumerate() {
            tx.execute(
                "INSERT INTO tasks.stacks (id, board_id, title, sort_order) VALUES ($1, $2, $3, $4)",
                params![kubuno_db::new_id(), board_id, *title, i as i32],
            )
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Niveau d'accès de l'utilisateur sur un board : "admin" si propriétaire,
    /// sinon la permission du partage, sinon None.
    pub async fn access_level(board_id: Uuid, user_id: Uuid, db: &DbPool) -> Result<Option<String>> {
        let owner: Option<Uuid> = db
            .fetch_optional_scalar("SELECT owner_id FROM tasks.boards WHERE id = $1", params![board_id])
            .await?;
        let owner = match owner {
            Some(o) => o,
            None => return Ok(None),
        };
        if owner == user_id {
            return Ok(Some("admin".to_string()));
        }
        let perm: Option<String> = db
            .fetch_optional_scalar(
                "SELECT permission FROM tasks.board_shares WHERE board_id = $1 AND shared_with = $2",
                params![board_id, user_id],
            )
            .await?;
        Ok(perm)
    }

    /// Vérifie que l'utilisateur a au moins la permission `min` sur le board.
    pub async fn assert_access(board_id: Uuid, user_id: Uuid, min: &str, db: &DbPool) -> Result<()> {
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

    pub async fn get(id: Uuid, user_id: Uuid, db: &DbPool) -> Result<Board> {
        Self::assert_access(id, user_id, "read", db).await?;
        db.fetch_optional_as::<Board>("SELECT * FROM tasks.boards WHERE id = $1", params![id])
            .await?
            .ok_or_else(|| TasksError::NotFound(format!("Board {id}")))
    }

    /// Refuses one more board when the account already sits at the ceiling the
    /// administrator set (`0` = no ceiling).
    pub async fn assert_can_create(
        user_id: Uuid,
        instance: &InstanceConfig,
        db: &DbPool,
    ) -> Result<()> {
        if instance.max_boards_per_user <= 0 {
            return Ok(());
        }
        let count_expr = db.backend().count_bigint("*");
        let owned: i64 = db
            .fetch_scalar(
                &format!("SELECT {count_expr} FROM tasks.boards WHERE owner_id = $1"),
                params![user_id],
            )
            .await?;
        if owned >= instance.max_boards_per_user {
            return Err(TasksError::Validation(format!(
                "Nombre maximal de tableaux atteint ({}) — supprimez-en un ou contactez votre administration",
                instance.max_boards_per_user
            )));
        }
        Ok(())
    }

    /// Crée un board ; pour un board kanban, ajoute trois colonnes par défaut.
    pub async fn create(user_id: Uuid, dto: CreateBoardDto, db: &DbPool) -> Result<Board> {
        let color      = dto.color.unwrap_or_else(|| "#1a73e8".to_string());
        let board_type = dto.board_type.unwrap_or_else(|| "kanban".to_string());
        if board_type != "kanban" && board_type != "list" {
            return Err(TasksError::Validation("board_type invalide".to_string()));
        }
        let board_id = dto.id.unwrap_or_else(kubuno_db::new_id);

        let mut tx = db.begin().await?;
        let seq = sync::next_board_seq(&mut tx).await?;
        tx.execute(
            "INSERT INTO tasks.boards
               (id, owner_id, title, description, color, board_type, caldav_token, ctag, change_seq)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            params![
                board_id, user_id, dto.title, dto.description, color, &board_type,
                sync::new_tag(), sync::new_tag(), seq
            ],
        )
        .await?;

        if board_type == "kanban" {
            for (i, title) in ["À faire", "En cours", "Terminé"].iter().enumerate() {
                // Client-minted stack ids (sync replay) are honoured in order.
                let sid = dto
                    .initial_stack_ids
                    .as_ref()
                    .and_then(|v| v.get(i))
                    .copied()
                    .unwrap_or_else(kubuno_db::new_id);
                tx.execute(
                    "INSERT INTO tasks.stacks (id, board_id, title, sort_order) VALUES ($1, $2, $3, $4)",
                    params![sid, board_id, *title, i as i32],
                )
                .await?;
            }
        }
        tx.commit().await?;

        db.fetch_one_as::<Board>("SELECT * FROM tasks.boards WHERE id = $1", params![board_id])
            .await
            .map_err(Into::into)
    }

    pub async fn update(id: Uuid, user_id: Uuid, dto: UpdateBoardDto, db: &DbPool) -> Result<Board> {
        Self::assert_access(id, user_id, "admin", db).await?;
        let current = db
            .fetch_optional_as::<Board>("SELECT * FROM tasks.boards WHERE id = $1", params![id])
            .await?
            .ok_or_else(|| TasksError::NotFound(format!("Board {id}")))?;

        // Le board par défaut ne peut être ni renommé ni archivé (toujours visible).
        let title       = if current.is_default { current.title.clone() } else { dto.title.unwrap_or(current.title) };
        let description = dto.description.or(current.description);
        let color       = dto.color.unwrap_or(current.color);
        let board_type  = dto.board_type.unwrap_or(current.board_type);
        let is_archived = if current.is_default { false } else { dto.is_archived.unwrap_or(current.is_archived) };
        let sort_order  = dto.sort_order.unwrap_or(current.sort_order);

        let mut tx = db.begin().await?;
        let seq = sync::next_board_seq(&mut tx).await?;
        tx.execute(
            "UPDATE tasks.boards
             SET title = $1, description = $2, color = $3, board_type = $4,
                 is_archived = $5, sort_order = $6, change_seq = $7
             WHERE id = $8",
            params![title, description, color, board_type, is_archived, sort_order, seq, id],
        )
        .await?;
        tx.commit().await?;

        db.fetch_one_as::<Board>("SELECT * FROM tasks.boards WHERE id = $1", params![id])
            .await
            .map_err(Into::into)
    }

    pub async fn delete(id: Uuid, user_id: Uuid, db: &DbPool) -> Result<()> {
        Self::assert_access(id, user_id, "admin", db).await?;
        let meta: Option<(Uuid, bool)> = db
            .fetch_optional_as("SELECT owner_id, is_default FROM tasks.boards WHERE id = $1", params![id])
            .await?;
        let (owner_id, is_default) = match meta {
            Some(m) => m,
            None => return Ok(()),
        };
        if is_default {
            return Err(TasksError::Conflict(
                "Le board par défaut ne peut pas être supprimé".to_string(),
            ));
        }

        // The FK cascade removes the board's tasks, but a cascade fires no
        // application code — so the delta feed would never learn those tasks are
        // gone. Record a task tombstone for each (owner-scoped, as tasks_delta
        // reads them) before the board goes, exactly as the old AFTER DELETE
        // trigger did on cascade.
        let doomed: Vec<(Uuid, Uuid)> = db
            .fetch_all_as("SELECT id, owner_id FROM tasks.tasks WHERE board_id = $1", params![id])
            .await?;

        let mut tx = db.begin().await?;
        for (task_id, task_owner) in doomed {
            let seq = sync::next_task_seq(&mut tx).await?;
            kubuno_db::journal::record_tombstone(&mut tx, sync::TASK_TOMBSTONES, task_id, task_owner, seq)
                .await?;
        }
        let seq = sync::next_board_seq(&mut tx).await?;
        tx.execute("DELETE FROM tasks.boards WHERE id = $1", params![id]).await?;
        kubuno_db::journal::record_tombstone(&mut tx, sync::BOARD_TOMBSTONES, id, owner_id, seq).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn share(
        id: Uuid,
        user_id: Uuid,
        dto: ShareBoardDto,
        instance: &InstanceConfig,
        db: &DbPool,
    ) -> Result<BoardShare> {
        if !instance.allow_board_sharing {
            return Err(TasksError::Validation(
                "Le partage d'un tableau est désactivé sur cette instance".to_string(),
            ));
        }
        Self::assert_access(id, user_id, "admin", db).await?;
        let is_default: bool = db
            .fetch_optional_scalar("SELECT is_default FROM tasks.boards WHERE id = $1", params![id])
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

        // Upsert on (board_id, shared_with); board shares are carried by neither
        // delta feed, so no change_seq is bumped.
        let clause = db.backend().upsert(
            "board_shares",
            &["board_id", "shared_with"],
            &[Assign::Incoming("permission")],
        );
        db.execute(
            &format!(
                "INSERT INTO tasks.board_shares (id, board_id, shared_with, permission)
                 VALUES ($1, $2, $3, $4){clause}"
            ),
            params![kubuno_db::new_id(), id, dto.user_id, permission],
        )
        .await?;

        db.fetch_one_as::<BoardShare>(
            "SELECT * FROM tasks.board_shares WHERE board_id = $1 AND shared_with = $2",
            params![id, dto.user_id],
        )
        .await
        .map_err(Into::into)
    }

    pub async fn unshare(id: Uuid, user_id: Uuid, shared_with: Uuid, db: &DbPool) -> Result<()> {
        Self::assert_access(id, user_id, "admin", db).await?;
        db.execute(
            "DELETE FROM tasks.board_shares WHERE board_id = $1 AND shared_with = $2",
            params![id, shared_with],
        )
        .await?;
        Ok(())
    }

    pub async fn list_shares(id: Uuid, user_id: Uuid, db: &DbPool) -> Result<Vec<BoardShare>> {
        Self::assert_access(id, user_id, "read", db).await?;
        let rows = db
            .fetch_all_as::<BoardShare>(
                "SELECT * FROM tasks.board_shares WHERE board_id = $1 ORDER BY created_at",
                params![id],
            )
            .await?;
        Ok(rows)
    }
}
