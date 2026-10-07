-- SQLite — `tasks` is an ATTACHed database file, attached on every pooled
-- connection by kubuno-db, so the qualified names below resolve as they do on
-- the other two engines. This single file declares the FINAL shape the
-- PostgreSQL side reached across 000001..000008.
--
-- Differences from PostgreSQL, and why:
--   * UUID -> BLOB, TIMESTAMPTZ -> TEXT (`%F %T%.f`, UTC), JSONB / UUID[] -> TEXT
--     (a JSON array), all as sqlx encodes/decodes them on SQLite.
--   * No DEFAULT on `id`: SQLite has no UUID generator; the process supplies it.
--   * updated_at and the CalDAV ctag are maintained by hand-written triggers
--     (SQLite has no ON UPDATE clause and no multi-event triggers). They do not
--     recurse: SQLite leaves recursive_triggers off.
--   * Partial indexes ARE kept (SQLite supports them), including the "one default
--     board per owner" unique index.
--   * Foreign-key REFERENCES are unqualified (SQLite assumes the same database).

CREATE TABLE tasks.boards (
    id           BLOB    NOT NULL PRIMARY KEY,
    owner_id     BLOB    NOT NULL,
    title        TEXT    NOT NULL,
    description  TEXT,
    color        TEXT    NOT NULL DEFAULT '#1a73e8',
    board_type   TEXT    NOT NULL DEFAULT 'kanban'
                     CHECK (board_type IN ('kanban', 'list')),
    is_default   INTEGER NOT NULL DEFAULT 0,
    is_archived  INTEGER NOT NULL DEFAULT 0,
    sort_order   INTEGER NOT NULL DEFAULT 0,
    caldav_token TEXT    NOT NULL UNIQUE,
    ctag         TEXT    NOT NULL,
    change_seq   INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    updated_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX tasks.idx_tasks_board_owner       ON boards(owner_id);
CREATE INDEX tasks.idx_tasks_board_token       ON boards(caldav_token);
CREATE INDEX tasks.idx_tasks_boards_change_seq ON boards(owner_id, change_seq);
CREATE UNIQUE INDEX tasks.idx_tasks_board_one_default ON boards(owner_id) WHERE is_default;

CREATE TABLE tasks.board_shares (
    id          BLOB NOT NULL PRIMARY KEY,
    board_id    BLOB NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    shared_with BLOB NOT NULL,
    permission  TEXT NOT NULL DEFAULT 'read'
                    CHECK (permission IN ('read', 'write', 'admin')),
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    UNIQUE (board_id, shared_with)
);
CREATE INDEX tasks.idx_tasks_bs_board ON board_shares(board_id);
CREATE INDEX tasks.idx_tasks_bs_user  ON board_shares(shared_with);

CREATE TABLE tasks.stacks (
    id         BLOB    NOT NULL PRIMARY KEY,
    board_id   BLOB    NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    title      TEXT    NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    updated_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX tasks.idx_tasks_stack_board ON stacks(board_id, sort_order);

CREATE TABLE tasks.labels (
    id         BLOB NOT NULL PRIMARY KEY,
    board_id   BLOB NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    title      TEXT NOT NULL,
    color      TEXT NOT NULL DEFAULT '#888888',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    UNIQUE (board_id, title)
);
CREATE INDEX tasks.idx_tasks_label_board ON labels(board_id);

CREATE TABLE tasks.tasks (
    id               BLOB    NOT NULL PRIMARY KEY,
    board_id         BLOB    NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    stack_id         BLOB    REFERENCES stacks(id) ON DELETE SET NULL,
    parent_task_id   BLOB    REFERENCES tasks(id) ON DELETE CASCADE,
    owner_id         BLOB    NOT NULL,
    title            TEXT    NOT NULL,
    description      TEXT,
    status           TEXT    NOT NULL DEFAULT 'open'
                         CHECK (status IN ('open', 'in_progress', 'done', 'cancelled')),
    priority         INTEGER NOT NULL DEFAULT 0 CHECK (priority BETWEEN 0 AND 9),
    percent_complete INTEGER NOT NULL DEFAULT 0 CHECK (percent_complete BETWEEN 0 AND 100),
    due_at           TEXT,
    start_at         TEXT,
    completed_at     TEXT,
    all_day          INTEGER NOT NULL DEFAULT 0,
    color            TEXT,
    rrule            TEXT,
    reminders        TEXT    NOT NULL,
    ical_uid         TEXT    NOT NULL UNIQUE,
    etag             TEXT    NOT NULL,
    sequence         INTEGER NOT NULL DEFAULT 0,
    sort_order       INTEGER NOT NULL DEFAULT 0,
    position         REAL    NOT NULL DEFAULT 0,
    linked_event_id  BLOB,
    linked_file_ids  TEXT    NOT NULL,
    starred          INTEGER NOT NULL DEFAULT 0,
    starred_at       TEXT,
    change_seq       INTEGER NOT NULL DEFAULT 0,
    created_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    updated_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    CONSTRAINT due_after_start CHECK (due_at IS NULL OR start_at IS NULL OR due_at >= start_at)
);
CREATE INDEX tasks.idx_tasks_task_board  ON tasks(board_id);
CREATE INDEX tasks.idx_tasks_task_stack  ON tasks(stack_id, position);
CREATE INDEX tasks.idx_tasks_task_owner  ON tasks(owner_id);
CREATE INDEX tasks.idx_tasks_task_parent ON tasks(parent_task_id) WHERE parent_task_id IS NOT NULL;
CREATE INDEX tasks.idx_tasks_task_due    ON tasks(due_at) WHERE due_at IS NOT NULL;
CREATE INDEX tasks.idx_tasks_task_uid    ON tasks(ical_uid);
CREATE INDEX tasks.idx_tasks_task_status ON tasks(status);
CREATE INDEX tasks.idx_tasks_task_event  ON tasks(linked_event_id) WHERE linked_event_id IS NOT NULL;
CREATE INDEX tasks.idx_tasks_starred     ON tasks(owner_id, starred_at) WHERE starred;
CREATE INDEX tasks.idx_tasks_tasks_change_seq ON tasks(owner_id, change_seq);

CREATE TABLE tasks.task_labels (
    task_id  BLOB NOT NULL REFERENCES tasks(id)  ON DELETE CASCADE,
    label_id BLOB NOT NULL REFERENCES labels(id) ON DELETE CASCADE,
    PRIMARY KEY (task_id, label_id)
);
CREATE INDEX tasks.idx_tasks_tl_label ON task_labels(label_id);

CREATE TABLE tasks.task_assignees (
    task_id    BLOB NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    user_id    BLOB NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    PRIMARY KEY (task_id, user_id)
);
CREATE INDEX tasks.idx_tasks_assignee_user ON task_assignees(user_id);

CREATE TABLE tasks.comments (
    id         BLOB NOT NULL PRIMARY KEY,
    task_id    BLOB NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    author_id  BLOB NOT NULL,
    body       TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX tasks.idx_tasks_comment_task ON comments(task_id, created_at);

CREATE TABLE tasks.attachments (
    id         BLOB NOT NULL PRIMARY KEY,
    task_id    BLOB NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    file_id    BLOB,
    filename   TEXT NOT NULL,
    mime_type  TEXT,
    size_bytes INTEGER,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX tasks.idx_tasks_attach_task ON attachments(task_id);

CREATE TABLE tasks.scheduled_reminders (
    id         BLOB NOT NULL PRIMARY KEY,
    task_id    BLOB NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    user_id    BLOB NOT NULL,
    remind_at  TEXT NOT NULL,
    channel    TEXT NOT NULL DEFAULT 'push'
                   CHECK (channel IN ('push', 'email', 'popup')),
    sent       INTEGER NOT NULL DEFAULT 0,
    sent_at    TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX tasks.idx_tasks_sr_remind ON scheduled_reminders(remind_at) WHERE sent = 0;

CREATE TABLE tasks.board_comments (
    id         BLOB NOT NULL PRIMARY KEY,
    board_id   BLOB NOT NULL REFERENCES boards(id) ON DELETE CASCADE,
    author_id  BLOB NOT NULL,
    body       TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX tasks.idx_tasks_bc_comments_board ON board_comments(board_id, created_at);

-- ── Delta journal: one shared counter, one tombstone table per entity ─────────

CREATE TABLE tasks.change_counter (
    domain TEXT   NOT NULL PRIMARY KEY,
    n      INTEGER NOT NULL
);

CREATE TABLE tasks.board_tombstones (
    id         BLOB    NOT NULL PRIMARY KEY,
    owner_id   BLOB    NOT NULL,
    change_seq INTEGER NOT NULL,
    deleted_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX tasks.idx_tasks_board_tomb_seq ON board_tombstones(owner_id, change_seq);

CREATE TABLE tasks.task_tombstones (
    id         BLOB    NOT NULL PRIMARY KEY,
    owner_id   BLOB    NOT NULL,
    change_seq INTEGER NOT NULL,
    deleted_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX tasks.idx_tasks_task_tomb_seq ON task_tombstones(owner_id, change_seq);

-- ── updated_at maintenance (hand-written; no recursion) ───────────────────────

CREATE TRIGGER tasks.boards_updated_at AFTER UPDATE ON boards
BEGIN
    UPDATE boards SET updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = NEW.id;
END;
CREATE TRIGGER tasks.stacks_updated_at AFTER UPDATE ON stacks
BEGIN
    UPDATE stacks SET updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = NEW.id;
END;
CREATE TRIGGER tasks.tasks_updated_at AFTER UPDATE ON tasks
BEGIN
    UPDATE tasks SET updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = NEW.id;
END;
CREATE TRIGGER tasks.comments_updated_at AFTER UPDATE ON comments
BEGIN
    UPDATE comments SET updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = NEW.id;
END;
CREATE TRIGGER tasks.board_comments_updated_at AFTER UPDATE ON board_comments
BEGIN
    UPDATE board_comments SET updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = NEW.id;
END;

-- ── CalDAV ctag: refreshed whenever a task in the board changes ───────────────

CREATE TRIGGER tasks.tasks_ctag_ins AFTER INSERT ON tasks
BEGIN
    UPDATE boards SET ctag = lower(hex(randomblob(16))) WHERE id = NEW.board_id;
END;
CREATE TRIGGER tasks.tasks_ctag_upd AFTER UPDATE ON tasks
BEGIN
    UPDATE boards SET ctag = lower(hex(randomblob(16))) WHERE id = NEW.board_id;
END;
CREATE TRIGGER tasks.tasks_ctag_del AFTER DELETE ON tasks
BEGIN
    UPDATE boards SET ctag = lower(hex(randomblob(16))) WHERE id = OLD.board_id;
END;
