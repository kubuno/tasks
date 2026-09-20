-- MySQL / MariaDB — the `tasks` database is created by kubuno-db's
-- `ensure_schema` before the migrator runs, so there is no CREATE DATABASE here.
-- This single file declares the FINAL shape the PostgreSQL side reached across
-- its 000001..000008 migrations (delta journal included, `linked_file_ids` as
-- JSON), stated once.
--
-- Differences from PostgreSQL, and why:
--   * UUID -> BINARY(16): what sqlx encodes a `uuid::Uuid` as on MySQL.
--   * No DEFAULT on `id`: MySQL has no gen_random_uuid() and no RETURNING, so
--     the process supplies every primary key.
--   * TIMESTAMPTZ -> DATETIME(6); every value written is UTC (the pool pins
--     `time_zone = '+00:00'`).
--   * JSONB / UUID[] -> JSON (reminders, linked_file_ids).
--   * updated_at is maintained by ON UPDATE CURRENT_TIMESTAMP(6).
--   * No partial indexes (MySQL has none): the "one default board per owner"
--     rule is enforced by the application; the other WHERE-filtered indexes
--     become plain indexes.
--   * The delta layer is the journal (change_counter + per-row change_seq); the
--     CalDAV ctag is refreshed by AFTER {INSERT,UPDATE,DELETE} triggers on tasks
--     (MySQL has no multi-event trigger, hence three).

CREATE TABLE tasks.boards (
    id           BINARY(16)   NOT NULL PRIMARY KEY,
    owner_id     BINARY(16)   NOT NULL,
    title        VARCHAR(255) NOT NULL,
    description  TEXT         NULL,
    color        VARCHAR(7)   NOT NULL DEFAULT '#1a73e8',
    board_type   VARCHAR(20)  NOT NULL DEFAULT 'kanban'
                     CHECK (board_type IN ('kanban', 'list')),
    is_default   BOOLEAN      NOT NULL DEFAULT FALSE,
    is_archived  BOOLEAN      NOT NULL DEFAULT FALSE,
    sort_order   INT          NOT NULL DEFAULT 0,
    caldav_token VARCHAR(64)  NOT NULL UNIQUE,
    ctag         VARCHAR(64)  NOT NULL,
    change_seq   BIGINT       NOT NULL DEFAULT 0,
    created_at   DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at   DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
                              ON UPDATE CURRENT_TIMESTAMP(6)
);
CREATE INDEX idx_tasks_board_owner      ON tasks.boards(owner_id);
CREATE INDEX idx_tasks_board_token      ON tasks.boards(caldav_token);
CREATE INDEX idx_tasks_boards_change_seq ON tasks.boards(owner_id, change_seq);

CREATE TABLE tasks.board_shares (
    id          BINARY(16)  NOT NULL PRIMARY KEY,
    board_id    BINARY(16)  NOT NULL,
    shared_with BINARY(16)  NOT NULL,
    permission  VARCHAR(20) NOT NULL DEFAULT 'read'
                    CHECK (permission IN ('read', 'write', 'admin')),
    created_at  DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    UNIQUE (board_id, shared_with),
    FOREIGN KEY (board_id) REFERENCES tasks.boards(id) ON DELETE CASCADE
);
CREATE INDEX idx_tasks_bs_board ON tasks.board_shares(board_id);
CREATE INDEX idx_tasks_bs_user  ON tasks.board_shares(shared_with);

CREATE TABLE tasks.stacks (
    id         BINARY(16)   NOT NULL PRIMARY KEY,
    board_id   BINARY(16)   NOT NULL,
    title      VARCHAR(255) NOT NULL,
    sort_order INT          NOT NULL DEFAULT 0,
    created_at DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
                            ON UPDATE CURRENT_TIMESTAMP(6),
    FOREIGN KEY (board_id) REFERENCES tasks.boards(id) ON DELETE CASCADE
);
CREATE INDEX idx_tasks_stack_board ON tasks.stacks(board_id, sort_order);

CREATE TABLE tasks.labels (
    id         BINARY(16)   NOT NULL PRIMARY KEY,
    board_id   BINARY(16)   NOT NULL,
    title      VARCHAR(100) NOT NULL,
    color      VARCHAR(7)   NOT NULL DEFAULT '#888888',
    created_at DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    UNIQUE (board_id, title),
    FOREIGN KEY (board_id) REFERENCES tasks.boards(id) ON DELETE CASCADE
);
CREATE INDEX idx_tasks_label_board ON tasks.labels(board_id);

CREATE TABLE tasks.tasks (
    id               BINARY(16)   NOT NULL PRIMARY KEY,
    board_id         BINARY(16)   NOT NULL,
    stack_id         BINARY(16)   NULL,
    parent_task_id   BINARY(16)   NULL,
    owner_id         BINARY(16)   NOT NULL,
    title            VARCHAR(500) NOT NULL,
    description      TEXT         NULL,
    status           VARCHAR(20)  NOT NULL DEFAULT 'open'
                         CHECK (status IN ('open', 'in_progress', 'done', 'cancelled')),
    priority         SMALLINT     NOT NULL DEFAULT 0 CHECK (priority BETWEEN 0 AND 9),
    percent_complete SMALLINT     NOT NULL DEFAULT 0 CHECK (percent_complete BETWEEN 0 AND 100),
    due_at           DATETIME(6)  NULL,
    start_at         DATETIME(6)  NULL,
    completed_at     DATETIME(6)  NULL,
    all_day          BOOLEAN      NOT NULL DEFAULT FALSE,
    color            VARCHAR(7)   NULL,
    rrule            TEXT         NULL,
    reminders        JSON         NOT NULL,
    ical_uid         VARCHAR(500) NOT NULL UNIQUE,
    etag             VARCHAR(64)  NOT NULL,
    sequence         INT          NOT NULL DEFAULT 0,
    sort_order       INT          NOT NULL DEFAULT 0,
    position         DOUBLE       NOT NULL DEFAULT 0,
    linked_event_id  BINARY(16)   NULL,
    linked_file_ids  JSON         NOT NULL,
    starred          BOOLEAN      NOT NULL DEFAULT FALSE,
    starred_at       DATETIME(6)  NULL,
    change_seq       BIGINT       NOT NULL DEFAULT 0,
    created_at       DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at       DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
                                  ON UPDATE CURRENT_TIMESTAMP(6),
    CONSTRAINT due_after_start CHECK (due_at IS NULL OR start_at IS NULL OR due_at >= start_at),
    FOREIGN KEY (board_id)       REFERENCES tasks.boards(id) ON DELETE CASCADE,
    FOREIGN KEY (stack_id)       REFERENCES tasks.stacks(id) ON DELETE SET NULL,
    FOREIGN KEY (parent_task_id) REFERENCES tasks.tasks(id) ON DELETE CASCADE
);
CREATE INDEX idx_tasks_task_board  ON tasks.tasks(board_id);
CREATE INDEX idx_tasks_task_stack  ON tasks.tasks(stack_id, position);
CREATE INDEX idx_tasks_task_owner  ON tasks.tasks(owner_id);
CREATE INDEX idx_tasks_task_parent ON tasks.tasks(parent_task_id);
CREATE INDEX idx_tasks_task_due    ON tasks.tasks(due_at);
CREATE INDEX idx_tasks_task_uid    ON tasks.tasks(ical_uid);
CREATE INDEX idx_tasks_task_status ON tasks.tasks(status);
CREATE INDEX idx_tasks_task_event  ON tasks.tasks(linked_event_id);
CREATE INDEX idx_tasks_starred     ON tasks.tasks(owner_id, starred_at);
CREATE INDEX idx_tasks_tasks_change_seq ON tasks.tasks(owner_id, change_seq);

CREATE TABLE tasks.task_labels (
    task_id  BINARY(16) NOT NULL,
    label_id BINARY(16) NOT NULL,
    PRIMARY KEY (task_id, label_id),
    FOREIGN KEY (task_id)  REFERENCES tasks.tasks(id)  ON DELETE CASCADE,
    FOREIGN KEY (label_id) REFERENCES tasks.labels(id) ON DELETE CASCADE
);
CREATE INDEX idx_tasks_tl_label ON tasks.task_labels(label_id);

CREATE TABLE tasks.task_assignees (
    task_id    BINARY(16)  NOT NULL,
    user_id    BINARY(16)  NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (task_id, user_id),
    FOREIGN KEY (task_id) REFERENCES tasks.tasks(id) ON DELETE CASCADE
);
CREATE INDEX idx_tasks_assignee_user ON tasks.task_assignees(user_id);

CREATE TABLE tasks.comments (
    id         BINARY(16)  NOT NULL PRIMARY KEY,
    task_id    BINARY(16)  NOT NULL,
    author_id  BINARY(16)  NOT NULL,
    body       TEXT        NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
                           ON UPDATE CURRENT_TIMESTAMP(6),
    FOREIGN KEY (task_id) REFERENCES tasks.tasks(id) ON DELETE CASCADE
);
CREATE INDEX idx_tasks_comment_task ON tasks.comments(task_id, created_at);

CREATE TABLE tasks.attachments (
    id         BINARY(16)   NOT NULL PRIMARY KEY,
    task_id    BINARY(16)   NOT NULL,
    file_id    BINARY(16)   NULL,
    filename   VARCHAR(500) NOT NULL,
    mime_type  VARCHAR(255) NULL,
    size_bytes BIGINT       NULL,
    created_at DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    FOREIGN KEY (task_id) REFERENCES tasks.tasks(id) ON DELETE CASCADE
);
CREATE INDEX idx_tasks_attach_task ON tasks.attachments(task_id);

CREATE TABLE tasks.scheduled_reminders (
    id         BINARY(16)  NOT NULL PRIMARY KEY,
    task_id    BINARY(16)  NOT NULL,
    user_id    BINARY(16)  NOT NULL,
    remind_at  DATETIME(6) NOT NULL,
    channel    VARCHAR(20) NOT NULL DEFAULT 'push'
                   CHECK (channel IN ('push', 'email', 'popup')),
    sent       BOOLEAN     NOT NULL DEFAULT FALSE,
    sent_at    DATETIME(6) NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    FOREIGN KEY (task_id) REFERENCES tasks.tasks(id) ON DELETE CASCADE
);
CREATE INDEX idx_tasks_sr_remind ON tasks.scheduled_reminders(remind_at);

CREATE TABLE tasks.board_comments (
    id         BINARY(16)  NOT NULL PRIMARY KEY,
    board_id   BINARY(16)  NOT NULL,
    author_id  BINARY(16)  NOT NULL,
    body       TEXT        NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
                           ON UPDATE CURRENT_TIMESTAMP(6),
    FOREIGN KEY (board_id) REFERENCES tasks.boards(id) ON DELETE CASCADE
);
CREATE INDEX idx_tasks_bc_comments_board ON tasks.board_comments(board_id, created_at);

-- ── Delta journal: one shared counter, one tombstone table per entity ─────────

CREATE TABLE tasks.change_counter (
    domain VARCHAR(190) NOT NULL PRIMARY KEY,
    n      BIGINT       NOT NULL
);

CREATE TABLE tasks.board_tombstones (
    id         BINARY(16)  NOT NULL PRIMARY KEY,
    owner_id   BINARY(16)  NOT NULL,
    change_seq BIGINT      NOT NULL,
    deleted_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
);
CREATE INDEX idx_tasks_board_tomb_seq ON tasks.board_tombstones(owner_id, change_seq);

CREATE TABLE tasks.task_tombstones (
    id         BINARY(16)  NOT NULL PRIMARY KEY,
    owner_id   BINARY(16)  NOT NULL,
    change_seq BIGINT      NOT NULL,
    deleted_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
);
CREATE INDEX idx_tasks_task_tomb_seq ON tasks.task_tombstones(owner_id, change_seq);

-- ── CalDAV ctag: refreshed whenever a task in the board changes ───────────────

CREATE TRIGGER tasks_ctag_ins AFTER INSERT ON tasks.tasks FOR EACH ROW
    UPDATE tasks.boards SET ctag = REPLACE(UUID(), '-', '') WHERE id = NEW.board_id;
CREATE TRIGGER tasks_ctag_upd AFTER UPDATE ON tasks.tasks FOR EACH ROW
    UPDATE tasks.boards SET ctag = REPLACE(UUID(), '-', '') WHERE id = NEW.board_id;
CREATE TRIGGER tasks_ctag_del AFTER DELETE ON tasks.tasks FOR EACH ROW
    UPDATE tasks.boards SET ctag = REPLACE(UUID(), '-', '') WHERE id = OLD.board_id;
