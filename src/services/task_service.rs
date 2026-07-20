use chrono::{Duration, Utc};
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use uuid::Uuid;

use crate::{
    errors::{Result, TasksError},
    models::{
        label::Label,
        task::{CreateTaskDto, MoveTaskDto, Task, TaskWithMeta, TasksQuery, UpdateTaskDto},
    },
    services::board_service::BoardService,
};

pub struct TaskService;

fn new_uid() -> String {
    format!("{}@kubuno.tasks", Uuid::new_v4())
}

impl TaskService {
    /// Board d'une tâche (pour les vérifications d'accès en cascade).
    pub async fn board_of_task(task_id: Uuid, db: &PgPool) -> Result<Uuid> {
        sqlx::query_scalar::<_, Uuid>("SELECT board_id FROM tasks.tasks WHERE id = $1")
            .bind(task_id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| TasksError::NotFound(format!("Task {task_id}")))
    }

    /// Accès à une tâche = accès au board, OU être assigné à la tâche.
    /// Un assigné dispose d'un accès « write » sur la tâche (même si le board ne
    /// lui est pas partagé — cas du board par défaut, non partageable mais dont
    /// les tâches peuvent l'être via l'attribution).
    pub async fn assert_task_access(task_id: Uuid, user_id: Uuid, min: &str, db: &PgPool) -> Result<()> {
        use crate::services::board_service::perm_rank;
        let board_id = Self::board_of_task(task_id, db).await?;

        if let Some(level) = BoardService::access_level(board_id, user_id, db).await? {
            if perm_rank(&level) >= perm_rank(min) {
                return Ok(());
            }
        }
        // Repli : assigné de la tâche → write.
        let assigned: Option<Uuid> = sqlx::query_scalar(
            "SELECT user_id FROM tasks.task_assignees WHERE task_id = $1 AND user_id = $2",
        )
        .bind(task_id)
        .bind(user_id)
        .fetch_optional(db)
        .await?;
        if assigned.is_some() && perm_rank("write") >= perm_rank(min) {
            return Ok(());
        }
        Err(TasksError::Forbidden)
    }

    /// Liste de tâches : par board/stack, ou via une collection intelligente
    /// (today/upcoming/overdue/important/completed/all) scopée aux boards accessibles.
    pub async fn list(user_id: Uuid, q: &TasksQuery, db: &PgPool) -> Result<Vec<Task>> {
        // Si filtré par board, vérifier l'accès en amont.
        if let Some(board_id) = q.board_id {
            BoardService::assert_access(board_id, user_id, "read", db).await?;
        }

        let mut qb: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT t.* FROM tasks.tasks t WHERE ");

        // Scope d'accès : tâche dans un board accessible, OU tâche qui m'est assignée.
        qb.push("(t.board_id IN (SELECT b.id FROM tasks.boards b LEFT JOIN tasks.board_shares bs ON bs.board_id = b.id WHERE b.owner_id = ");
        qb.push_bind(user_id);
        qb.push(" OR bs.shared_with = ");
        qb.push_bind(user_id);
        qb.push(") OR EXISTS (SELECT 1 FROM tasks.task_assignees ta_s WHERE ta_s.task_id = t.id AND ta_s.user_id = ");
        qb.push_bind(user_id);
        qb.push("))");

        if let Some(board_id) = q.board_id {
            qb.push(" AND t.board_id = ");
            qb.push_bind(board_id);
        }
        if let Some(stack_id) = q.stack_id {
            qb.push(" AND t.stack_id = ");
            qb.push_bind(stack_id);
        }
        if let Some(ref status) = q.status {
            qb.push(" AND t.status = ");
            qb.push_bind(status.clone());
        }
        if let Some(assignee) = q.assignee {
            qb.push(" AND EXISTS (SELECT 1 FROM tasks.task_assignees ta WHERE ta.task_id = t.id AND ta.user_id = ");
            qb.push_bind(assignee);
            qb.push(")");
        }
        if let Some(label_id) = q.label_id {
            qb.push(" AND EXISTS (SELECT 1 FROM tasks.task_labels tl WHERE tl.task_id = t.id AND tl.label_id = ");
            qb.push_bind(label_id);
            qb.push(")");
        }
        if let Some(ref search) = q.search {
            qb.push(" AND (t.title ILIKE ");
            qb.push_bind(format!("%{search}%"));
            qb.push(" OR t.description ILIKE ");
            qb.push_bind(format!("%{search}%"));
            qb.push(")");
        }
        if let Some(due_before) = q.due_before {
            qb.push(" AND t.due_at <= ");
            qb.push_bind(due_before);
        }
        if let Some(due_after) = q.due_after {
            qb.push(" AND t.due_at >= ");
            qb.push_bind(due_after);
        }

        // Collections intelligentes.
        match q.collection.as_deref() {
            Some("today") => {
                qb.push(" AND t.due_at::date = CURRENT_DATE AND t.status NOT IN ('done','cancelled')");
            }
            Some("upcoming") => {
                qb.push(" AND t.due_at > NOW() AND t.due_at <= NOW() + INTERVAL '7 days' AND t.status NOT IN ('done','cancelled')");
            }
            Some("overdue") => {
                qb.push(" AND t.due_at < NOW() AND t.status NOT IN ('done','cancelled')");
            }
            Some("important") => {
                qb.push(" AND t.priority >= 6 AND t.status NOT IN ('done','cancelled')");
            }
            Some("completed") => {
                qb.push(" AND t.status = 'done'");
            }
            Some("all") | None => {}
            Some(other) => {
                return Err(TasksError::Validation(format!("collection inconnue: {other}")));
            }
        }

        // Par défaut on ne renvoie que les tâches racines (sauf demande explicite).
        if !q.include_subtasks {
            qb.push(" AND t.parent_task_id IS NULL");
        }

        qb.push(" ORDER BY t.position ASC, t.sort_order ASC, t.created_at ASC");

        let rows = qb.build_query_as::<Task>().fetch_all(db).await?;
        Ok(rows)
    }

    pub async fn get(id: Uuid, user_id: Uuid, db: &PgPool) -> Result<Task> {
        Self::assert_task_access(id, user_id, "read", db).await?;
        sqlx::query_as::<_, Task>("SELECT * FROM tasks.tasks WHERE id = $1")
            .bind(id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| TasksError::NotFound(format!("Task {id}")))
    }

    pub async fn get_with_meta(id: Uuid, user_id: Uuid, db: &PgPool) -> Result<TaskWithMeta> {
        let task = Self::get(id, user_id, db).await?;
        Self::enrich(task, db).await
    }

    async fn enrich(task: Task, db: &PgPool) -> Result<TaskWithMeta> {
        let labels = sqlx::query_as::<_, Label>(
            r#"
            SELECT l.* FROM tasks.labels l
            JOIN tasks.task_labels tl ON tl.label_id = l.id
            WHERE tl.task_id = $1
            ORDER BY l.title
            "#,
        )
        .bind(task.id)
        .fetch_all(db)
        .await?;

        let assignees: Vec<Uuid> = sqlx::query_scalar(
            "SELECT user_id FROM tasks.task_assignees WHERE task_id = $1",
        )
        .bind(task.id)
        .fetch_all(db)
        .await?;

        let subtask_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tasks.tasks WHERE parent_task_id = $1")
                .bind(task.id)
                .fetch_one(db)
                .await?;

        let comment_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tasks.comments WHERE task_id = $1")
                .bind(task.id)
                .fetch_one(db)
                .await?;

        Ok(TaskWithMeta { task, labels, assignees, subtask_count, comment_count })
    }

    pub async fn create(user_id: Uuid, dto: CreateTaskDto, db: &PgPool) -> Result<TaskWithMeta> {
        BoardService::assert_access(dto.board_id, user_id, "write", db).await?;

        // Cohérence stack ↔ board.
        if let Some(stack_id) = dto.stack_id {
            let ok: Option<Uuid> = sqlx::query_scalar(
                "SELECT id FROM tasks.stacks WHERE id = $1 AND board_id = $2",
            )
            .bind(stack_id)
            .bind(dto.board_id)
            .fetch_optional(db)
            .await?;
            if ok.is_none() {
                return Err(TasksError::Validation("stack_id n'appartient pas au board".into()));
            }
        }

        let status   = dto.status.unwrap_or_else(|| "open".to_string());
        if !["open", "in_progress", "done", "cancelled"].contains(&status.as_str()) {
            return Err(TasksError::Validation("status invalide".into()));
        }
        let priority = dto.priority.unwrap_or(0);
        let (percent, completed_at) = if status == "done" {
            (100i16, Some(Utc::now()))
        } else {
            (dto.percent_complete.unwrap_or(0), None)
        };
        let all_day   = dto.all_day.unwrap_or(false);
        let reminders = dto.reminders.unwrap_or_else(|| serde_json::json!([]));
        let uid       = new_uid();

        // Position en fin de colonne/board.
        let position: f64 = sqlx::query_scalar::<_, Option<f64>>(
            "SELECT MAX(position) FROM tasks.tasks WHERE board_id = $1 AND stack_id IS NOT DISTINCT FROM $2",
        )
        .bind(dto.board_id)
        .bind(dto.stack_id)
        .fetch_one(db)
        .await?
        .map(|m| m + 1.0)
        .unwrap_or(0.0);

        let mut tx = db.begin().await?;

        let task = sqlx::query_as::<_, Task>(
            r#"
            INSERT INTO tasks.tasks
                (id, board_id, stack_id, parent_task_id, owner_id, title, description,
                 status, priority, percent_complete, due_at, start_at, completed_at,
                 all_day, color, rrule, reminders, ical_uid, position, linked_event_id)
            VALUES (COALESCE($20, uuid_generate_v4()),$1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19)
            RETURNING *
            "#,
        )
        .bind(dto.board_id)
        .bind(dto.stack_id)
        .bind(dto.parent_task_id)
        .bind(user_id)
        .bind(&dto.title)
        .bind(&dto.description)
        .bind(&status)
        .bind(priority)
        .bind(percent)
        .bind(dto.due_at)
        .bind(dto.start_at)
        .bind(completed_at)
        .bind(all_day)
        .bind(&dto.color)
        .bind(&dto.rrule)
        .bind(&reminders)
        .bind(&uid)
        .bind(position)
        .bind(dto.linked_event_id)
        .bind(dto.id)
        .fetch_one(&mut *tx)
        .await?;

        if let Some(ref ids) = dto.label_ids {
            Self::set_labels(&mut tx, task.id, dto.board_id, ids).await?;
        }
        if let Some(ref ids) = dto.assignee_ids {
            Self::set_assignees(&mut tx, task.id, ids).await?;
        }

        tx.commit().await?;

        if let Some(ref ids) = dto.assignee_ids {
            for a in ids {
                Self::ensure_share_for_assignee(dto.board_id, user_id, *a, db).await?;
            }
        }

        Self::schedule_reminders(&task, user_id, db).await?;
        Self::enrich(task, db).await
    }

    pub async fn update(id: Uuid, user_id: Uuid, dto: UpdateTaskDto, db: &PgPool) -> Result<TaskWithMeta> {
        Self::assert_task_access(id, user_id, "write", db).await?;
        let board_id = Self::board_of_task(id, db).await?;

        let cur = sqlx::query_as::<_, Task>("SELECT * FROM tasks.tasks WHERE id = $1")
            .bind(id)
            .fetch_one(db)
            .await?;

        let title       = dto.title.unwrap_or(cur.title);
        let description = dto.description.or(cur.description);
        let status      = dto.status.unwrap_or(cur.status.clone());
        if !["open", "in_progress", "done", "cancelled"].contains(&status.as_str()) {
            return Err(TasksError::Validation("status invalide".into()));
        }
        let priority    = dto.priority.unwrap_or(cur.priority);
        let all_day     = dto.all_day.unwrap_or(cur.all_day);
        let color       = if dto.clear_color { None } else { dto.color.or(cur.color) };
        let rrule       = dto.rrule.or(cur.rrule);
        let reminders   = dto.reminders.unwrap_or(cur.reminders);
        let due_at      = dto.due_at.or(cur.due_at);
        let start_at    = dto.start_at.or(cur.start_at);
        let stack_id    = dto.stack_id.or(cur.stack_id);
        let parent_task_id = dto.parent_task_id.or(cur.parent_task_id);

        // Gestion du % et de la date de complétion selon le statut.
        let (percent, completed_at) = if status == "done" {
            (dto.percent_complete.unwrap_or(100), cur.completed_at.or(Some(Utc::now())))
        } else {
            (dto.percent_complete.unwrap_or(cur.percent_complete), None)
        };

        let linked_event_id = if dto.clear_linked_event {
            None
        } else {
            dto.linked_event_id.or(cur.linked_event_id)
        };

        let mut tx = db.begin().await?;

        let task = sqlx::query_as::<_, Task>(
            r#"
            UPDATE tasks.tasks
            SET stack_id = $2, parent_task_id = $3, title = $4, description = $5,
                status = $6, priority = $7, percent_complete = $8, due_at = $9,
                start_at = $10, completed_at = $11, all_day = $12, rrule = $13,
                reminders = $14, linked_event_id = $15, color = $16,
                sequence = sequence + 1, etag = md5(random()::text)
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(stack_id)
        .bind(parent_task_id)
        .bind(&title)
        .bind(&description)
        .bind(&status)
        .bind(priority)
        .bind(percent)
        .bind(due_at)
        .bind(start_at)
        .bind(completed_at)
        .bind(all_day)
        .bind(&rrule)
        .bind(&reminders)
        .bind(linked_event_id)
        .bind(&color)
        .fetch_one(&mut *tx)
        .await?;

        if let Some(ref ids) = dto.label_ids {
            Self::set_labels(&mut tx, id, board_id, ids).await?;
        }
        if let Some(ref ids) = dto.assignee_ids {
            Self::set_assignees(&mut tx, id, ids).await?;
        }

        tx.commit().await?;

        if let Some(ref ids) = dto.assignee_ids {
            for a in ids {
                Self::ensure_share_for_assignee(board_id, user_id, *a, db).await?;
            }
        }

        Self::schedule_reminders(&task, user_id, db).await?;
        Self::enrich(task, db).await
    }

    /// Marque une tâche comme terminée (statut done + 100%).
    pub async fn complete(id: Uuid, user_id: Uuid, db: &PgPool) -> Result<Task> {
        Self::assert_task_access(id, user_id, "write", db).await?;
        let task = sqlx::query_as::<_, Task>(
            r#"
            UPDATE tasks.tasks
            SET status = 'done', percent_complete = 100, completed_at = NOW(),
                sequence = sequence + 1, etag = md5(random()::text)
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(db)
        .await?;
        Ok(task)
    }

    pub async fn move_task(id: Uuid, user_id: Uuid, dto: MoveTaskDto, db: &PgPool) -> Result<Task> {
        let board_id = Self::board_of_task(id, db).await?;
        BoardService::assert_access(board_id, user_id, "write", db).await?;

        if let Some(stack_id) = dto.stack_id {
            let ok: Option<Uuid> = sqlx::query_scalar(
                "SELECT id FROM tasks.stacks WHERE id = $1 AND board_id = $2",
            )
            .bind(stack_id)
            .bind(board_id)
            .fetch_optional(db)
            .await?;
            if ok.is_none() {
                return Err(TasksError::Validation("stack cible hors du board".into()));
            }
        }

        let sort_order = dto.sort_order.unwrap_or(0);
        let task = sqlx::query_as::<_, Task>(
            r#"
            UPDATE tasks.tasks
            SET stack_id = $2, position = $3, sort_order = $4, etag = md5(random()::text)
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(dto.stack_id)
        .bind(dto.position)
        .bind(sort_order)
        .fetch_one(db)
        .await?;
        Ok(task)
    }

    pub async fn delete(id: Uuid, user_id: Uuid, db: &PgPool) -> Result<()> {
        let board_id = Self::board_of_task(id, db).await?;
        BoardService::assert_access(board_id, user_id, "write", db).await?;
        sqlx::query("DELETE FROM tasks.tasks WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;
        Ok(())
    }

    /// Déplace une ou plusieurs tâches (et leurs sous-tâches) vers un autre board.
    /// Les labels — propres à un board — sont détachés. Nécessite l'accès write
    /// sur le board cible ET sur le board source de chaque tâche.
    /// Retourne les ids des tâches racines effectivement déplacées.
    pub async fn move_to_board(
        user_id: Uuid,
        dto: crate::models::task::MoveToBoardDto,
        db: &PgPool,
    ) -> Result<Vec<Uuid>> {
        BoardService::assert_access(dto.target_board_id, user_id, "write", db).await?;

        // Colonne cible : celle fournie (validée), sinon la 1ʳᵉ du board cible.
        let target_stack: Option<Uuid> = match dto.target_stack_id {
            Some(s) => {
                let ok: Option<Uuid> = sqlx::query_scalar(
                    "SELECT id FROM tasks.stacks WHERE id = $1 AND board_id = $2",
                )
                .bind(s)
                .bind(dto.target_board_id)
                .fetch_optional(db)
                .await?;
                if ok.is_none() {
                    return Err(TasksError::Validation("colonne cible hors du board".into()));
                }
                Some(s)
            }
            None => sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM tasks.stacks WHERE board_id = $1 ORDER BY sort_order, created_at LIMIT 1",
            )
            .bind(dto.target_board_id)
            .fetch_optional(db)
            .await?,
        };

        let mut tx = db.begin().await?;
        let mut moved = Vec::new();

        for task_id in &dto.task_ids {
            // La tâche doit exister et l'utilisateur avoir write sur son board source.
            let src_board: Option<Uuid> =
                sqlx::query_scalar("SELECT board_id FROM tasks.tasks WHERE id = $1 AND parent_task_id IS NULL")
                    .bind(task_id)
                    .fetch_optional(&mut *tx)
                    .await?;
            let src_board = match src_board {
                Some(b) => b,
                None => continue, // tâche inconnue ou sous-tâche : ignorée
            };
            if src_board == dto.target_board_id {
                continue; // déjà sur le board cible
            }
            if BoardService::assert_access(src_board, user_id, "write", db).await.is_err() {
                continue;
            }

            // Tâche + toutes ses descendantes.
            let ids: Vec<Uuid> = sqlx::query_scalar(
                r#"
                WITH RECURSIVE sub AS (
                    SELECT id FROM tasks.tasks WHERE id = $1
                    UNION ALL
                    SELECT t.id FROM tasks.tasks t JOIN sub ON t.parent_task_id = sub.id
                )
                SELECT id FROM sub
                "#,
            )
            .bind(task_id)
            .fetch_all(&mut *tx)
            .await?;

            // Détacher les labels (propres au board source).
            sqlx::query("DELETE FROM tasks.task_labels WHERE task_id = ANY($1)")
                .bind(&ids)
                .execute(&mut *tx)
                .await?;

            // Position en fin de colonne cible.
            let position: f64 = sqlx::query_scalar::<_, Option<f64>>(
                "SELECT MAX(position) FROM tasks.tasks WHERE board_id = $1 AND stack_id IS NOT DISTINCT FROM $2",
            )
            .bind(dto.target_board_id)
            .bind(target_stack)
            .fetch_one(&mut *tx)
            .await?
            .map(|m| m + 1.0)
            .unwrap_or(0.0);

            // Racine → colonne cible ; descendantes → board cible, sans colonne.
            sqlx::query(
                r#"
                UPDATE tasks.tasks
                SET board_id = $2,
                    stack_id = CASE WHEN id = $3 THEN $4 ELSE NULL END,
                    position = CASE WHEN id = $3 THEN $5 ELSE position END,
                    etag = md5(random()::text)
                WHERE id = ANY($1)
                "#,
            )
            .bind(&ids)
            .bind(dto.target_board_id)
            .bind(task_id)
            .bind(target_stack)
            .bind(position)
            .execute(&mut *tx)
            .await?;

            moved.push(*task_id);
        }

        tx.commit().await?;
        Ok(moved)
    }

    pub async fn list_subtasks(parent_id: Uuid, user_id: Uuid, db: &PgPool) -> Result<Vec<Task>> {
        Self::assert_task_access(parent_id, user_id, "read", db).await?;
        let rows = sqlx::query_as::<_, Task>(
            "SELECT * FROM tasks.tasks WHERE parent_task_id = $1 ORDER BY position, created_at",
        )
        .bind(parent_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    // ── Assignés ────────────────────────────────────────────────────────────────

    /// Crée naturellement le partage du board quand on assigne une tâche à
    /// quelqu'un d'autre — SAUF : board par défaut (non partageable), assignation
    /// à soi-même, ou au propriétaire du board.
    async fn ensure_share_for_assignee(
        board_id: Uuid,
        acting_user: Uuid,
        assignee: Uuid,
        db: &PgPool,
    ) -> Result<()> {
        let board: Option<(Uuid, bool)> =
            sqlx::query_as("SELECT owner_id, is_default FROM tasks.boards WHERE id = $1")
                .bind(board_id)
                .fetch_optional(db)
                .await?;
        let (owner_id, is_default) = match board {
            Some(b) => b,
            None => return Ok(()),
        };
        if is_default || assignee == acting_user || assignee == owner_id {
            return Ok(());
        }
        sqlx::query(
            r#"
            INSERT INTO tasks.board_shares (board_id, shared_with, permission)
            VALUES ($1, $2, 'write')
            ON CONFLICT (board_id, shared_with) DO NOTHING
            "#,
        )
        .bind(board_id)
        .bind(assignee)
        .execute(db)
        .await?;
        Ok(())
    }

    pub async fn add_assignee(task_id: Uuid, user_id: Uuid, assignee: Uuid, db: &PgPool) -> Result<()> {
        Self::assert_task_access(task_id, user_id, "write", db).await?;
        let board_id = Self::board_of_task(task_id, db).await?;

        let mut tx = db.begin().await?;
        sqlx::query(
            "INSERT INTO tasks.task_assignees (task_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(task_id)
        .bind(assignee)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        // Partage naturel du board (hors board par défaut / soi-même / propriétaire).
        Self::ensure_share_for_assignee(board_id, user_id, assignee, db).await?;
        Ok(())
    }

    pub async fn remove_assignee(task_id: Uuid, user_id: Uuid, assignee: Uuid, db: &PgPool) -> Result<()> {
        Self::assert_task_access(task_id, user_id, "write", db).await?;
        sqlx::query("DELETE FROM tasks.task_assignees WHERE task_id = $1 AND user_id = $2")
            .bind(task_id)
            .bind(assignee)
            .execute(db)
            .await?;
        Ok(())
    }

    pub async fn list_assignees(task_id: Uuid, user_id: Uuid, db: &PgPool) -> Result<Vec<Uuid>> {
        Self::assert_task_access(task_id, user_id, "read", db).await?;
        let rows = sqlx::query_scalar::<_, Uuid>(
            "SELECT user_id FROM tasks.task_assignees WHERE task_id = $1",
        )
        .bind(task_id)
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    // ── Labels sur une tâche ──────────────────────────────────────────────────────

    pub async fn add_label(task_id: Uuid, user_id: Uuid, label_id: Uuid, db: &PgPool) -> Result<()> {
        let board_id = Self::board_of_task(task_id, db).await?;
        BoardService::assert_access(board_id, user_id, "write", db).await?;
        // Le label doit appartenir au même board.
        let ok: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM tasks.labels WHERE id = $1 AND board_id = $2")
                .bind(label_id)
                .bind(board_id)
                .fetch_optional(db)
                .await?;
        if ok.is_none() {
            return Err(TasksError::Validation("label hors du board".into()));
        }
        sqlx::query(
            "INSERT INTO tasks.task_labels (task_id, label_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(task_id)
        .bind(label_id)
        .execute(db)
        .await?;
        Ok(())
    }

    pub async fn remove_label(task_id: Uuid, user_id: Uuid, label_id: Uuid, db: &PgPool) -> Result<()> {
        let board_id = Self::board_of_task(task_id, db).await?;
        BoardService::assert_access(board_id, user_id, "write", db).await?;
        sqlx::query("DELETE FROM tasks.task_labels WHERE task_id = $1 AND label_id = $2")
            .bind(task_id)
            .bind(label_id)
            .execute(db)
            .await?;
        Ok(())
    }

    // ── Helpers tx ────────────────────────────────────────────────────────────────

    async fn set_labels(
        tx: &mut Transaction<'_, Postgres>,
        task_id: Uuid,
        board_id: Uuid,
        label_ids: &[Uuid],
    ) -> Result<()> {
        sqlx::query("DELETE FROM tasks.task_labels WHERE task_id = $1")
            .bind(task_id)
            .execute(&mut **tx)
            .await?;
        for lid in label_ids {
            // Ignore les labels hors du board.
            sqlx::query(
                r#"
                INSERT INTO tasks.task_labels (task_id, label_id)
                SELECT $1, $2 WHERE EXISTS (
                    SELECT 1 FROM tasks.labels WHERE id = $2 AND board_id = $3
                )
                ON CONFLICT DO NOTHING
                "#,
            )
            .bind(task_id)
            .bind(lid)
            .bind(board_id)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    async fn set_assignees(
        tx: &mut Transaction<'_, Postgres>,
        task_id: Uuid,
        assignee_ids: &[Uuid],
    ) -> Result<()> {
        sqlx::query("DELETE FROM tasks.task_assignees WHERE task_id = $1")
            .bind(task_id)
            .execute(&mut **tx)
            .await?;
        for uid in assignee_ids {
            sqlx::query(
                "INSERT INTO tasks.task_assignees (task_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(task_id)
            .bind(uid)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    /// (Re)planifie les rappels d'une tâche en fonction de son échéance.
    async fn schedule_reminders(task: &Task, user_id: Uuid, db: &PgPool) -> Result<()> {
        sqlx::query("DELETE FROM tasks.scheduled_reminders WHERE task_id = $1 AND sent = FALSE")
            .bind(task.id)
            .execute(db)
            .await?;

        let due = match task.due_at {
            Some(d) => d,
            None => return Ok(()),
        };
        let reminders = match task.reminders.as_array() {
            Some(arr) => arr.clone(),
            None => return Ok(()),
        };
        for reminder in reminders {
            let minutes_before = reminder.get("minutes_before").and_then(|v| v.as_i64()).unwrap_or(15);
            let channel = reminder.get("type").and_then(|v| v.as_str()).unwrap_or("push").to_string();
            let remind_at = due - Duration::minutes(minutes_before);
            if remind_at > Utc::now() {
                sqlx::query(
                    r#"
                    INSERT INTO tasks.scheduled_reminders (task_id, user_id, remind_at, channel)
                    VALUES ($1, $2, $3, $4)
                    ON CONFLICT DO NOTHING
                    "#,
                )
                .bind(task.id)
                .bind(user_id)
                .bind(remind_at)
                .bind(&channel)
                .execute(db)
                .await?;
            }
        }
        Ok(())
    }
}
