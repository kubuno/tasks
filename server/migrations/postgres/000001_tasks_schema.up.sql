-- Module Tasks — schéma `tasks` (Tasks + Deck)
-- Le schéma est créé par main.rs ; les migrations tournent avec search_path = tasks,public.

-- ── Fonctions trigger ─────────────────────────────────────────────────────────

CREATE OR REPLACE FUNCTION tasks.set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Régénère le ctag du board à chaque mutation d'une tâche, pour que les
-- clients CalDAV détectent un changement et resynchronisent.
CREATE OR REPLACE FUNCTION tasks.bump_board_ctag()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE tasks.boards
       SET ctag = md5(random()::text || clock_timestamp()::text)
     WHERE id = COALESCE(NEW.board_id, OLD.board_id);
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

-- ── Boards (liste Tasks / tableau Deck unifié) ────────────────────────────────

CREATE TABLE tasks.boards (
    id           UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    owner_id     UUID NOT NULL,
    title        VARCHAR(255) NOT NULL,
    description  TEXT,
    color        VARCHAR(7) NOT NULL DEFAULT '#1a73e8',
    board_type   VARCHAR(20) NOT NULL DEFAULT 'kanban'
                     CHECK (board_type IN ('kanban', 'list')),
    is_archived  BOOLEAN NOT NULL DEFAULT FALSE,
    sort_order   INTEGER NOT NULL DEFAULT 0,
    caldav_token VARCHAR(64) UNIQUE NOT NULL DEFAULT md5(random()::text || clock_timestamp()::text),
    ctag         VARCHAR(64) NOT NULL DEFAULT md5(random()::text),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_tasks_board_owner ON tasks.boards(owner_id);
CREATE INDEX idx_tasks_board_token ON tasks.boards(caldav_token);

CREATE TABLE tasks.board_shares (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    board_id    UUID NOT NULL REFERENCES tasks.boards(id) ON DELETE CASCADE,
    shared_with UUID NOT NULL,
    permission  VARCHAR(20) NOT NULL DEFAULT 'read'
                    CHECK (permission IN ('read', 'write', 'admin')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (board_id, shared_with)
);
CREATE INDEX idx_tasks_bs_board ON tasks.board_shares(board_id);
CREATE INDEX idx_tasks_bs_user  ON tasks.board_shares(shared_with);

-- ── Stacks (colonnes Kanban) ──────────────────────────────────────────────────

CREATE TABLE tasks.stacks (
    id         UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    board_id   UUID NOT NULL REFERENCES tasks.boards(id) ON DELETE CASCADE,
    title      VARCHAR(255) NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_tasks_stack_board ON tasks.stacks(board_id, sort_order);

-- ── Labels ────────────────────────────────────────────────────────────────────

CREATE TABLE tasks.labels (
    id         UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    board_id   UUID NOT NULL REFERENCES tasks.boards(id) ON DELETE CASCADE,
    title      VARCHAR(100) NOT NULL,
    color      VARCHAR(7) NOT NULL DEFAULT '#888888',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (board_id, title)
);
CREATE INDEX idx_tasks_label_board ON tasks.labels(board_id);

-- ── Tasks (cartes) ────────────────────────────────────────────────────────────

CREATE TABLE tasks.tasks (
    id               UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    board_id         UUID NOT NULL REFERENCES tasks.boards(id) ON DELETE CASCADE,
    stack_id         UUID REFERENCES tasks.stacks(id) ON DELETE SET NULL,
    parent_task_id   UUID REFERENCES tasks.tasks(id) ON DELETE CASCADE,
    owner_id         UUID NOT NULL,
    title            VARCHAR(500) NOT NULL,
    description      TEXT,
    status           VARCHAR(20) NOT NULL DEFAULT 'open'
                         CHECK (status IN ('open', 'in_progress', 'done', 'cancelled')),
    priority         SMALLINT NOT NULL DEFAULT 0 CHECK (priority BETWEEN 0 AND 9),
    percent_complete SMALLINT NOT NULL DEFAULT 0 CHECK (percent_complete BETWEEN 0 AND 100),
    due_at           TIMESTAMPTZ,
    start_at         TIMESTAMPTZ,
    completed_at     TIMESTAMPTZ,
    all_day          BOOLEAN NOT NULL DEFAULT FALSE,
    rrule            TEXT,
    reminders        JSONB NOT NULL DEFAULT '[]',
    ical_uid         VARCHAR(500) UNIQUE NOT NULL,
    etag             VARCHAR(64) NOT NULL DEFAULT md5(random()::text),
    sequence         INTEGER NOT NULL DEFAULT 0,
    sort_order       INTEGER NOT NULL DEFAULT 0,
    position         DOUBLE PRECISION NOT NULL DEFAULT 0,
    linked_event_id  UUID,
    linked_file_ids  UUID[] NOT NULL DEFAULT '{}',
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT due_after_start CHECK (due_at IS NULL OR start_at IS NULL OR due_at >= start_at)
);
CREATE INDEX idx_tasks_task_board  ON tasks.tasks(board_id);
CREATE INDEX idx_tasks_task_stack  ON tasks.tasks(stack_id, position);
CREATE INDEX idx_tasks_task_owner  ON tasks.tasks(owner_id);
CREATE INDEX idx_tasks_task_parent ON tasks.tasks(parent_task_id) WHERE parent_task_id IS NOT NULL;
CREATE INDEX idx_tasks_task_due    ON tasks.tasks(due_at) WHERE due_at IS NOT NULL;
CREATE INDEX idx_tasks_task_uid    ON tasks.tasks(ical_uid);
CREATE INDEX idx_tasks_task_status ON tasks.tasks(status);
CREATE INDEX idx_tasks_task_event  ON tasks.tasks(linked_event_id) WHERE linked_event_id IS NOT NULL;

CREATE TABLE tasks.task_labels (
    task_id  UUID NOT NULL REFERENCES tasks.tasks(id) ON DELETE CASCADE,
    label_id UUID NOT NULL REFERENCES tasks.labels(id) ON DELETE CASCADE,
    PRIMARY KEY (task_id, label_id)
);
CREATE INDEX idx_tasks_tl_label ON tasks.task_labels(label_id);

CREATE TABLE tasks.task_assignees (
    task_id    UUID NOT NULL REFERENCES tasks.tasks(id) ON DELETE CASCADE,
    user_id    UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (task_id, user_id)
);
CREATE INDEX idx_tasks_assignee_user ON tasks.task_assignees(user_id);

CREATE TABLE tasks.comments (
    id         UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    task_id    UUID NOT NULL REFERENCES tasks.tasks(id) ON DELETE CASCADE,
    author_id  UUID NOT NULL,
    body       TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_tasks_comment_task ON tasks.comments(task_id, created_at);

CREATE TABLE tasks.attachments (
    id         UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    task_id    UUID NOT NULL REFERENCES tasks.tasks(id) ON DELETE CASCADE,
    file_id    UUID,
    filename   VARCHAR(500) NOT NULL,
    mime_type  VARCHAR(255),
    size_bytes BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_tasks_attach_task ON tasks.attachments(task_id);

CREATE TABLE tasks.scheduled_reminders (
    id         UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    task_id    UUID NOT NULL REFERENCES tasks.tasks(id) ON DELETE CASCADE,
    user_id    UUID NOT NULL,
    remind_at  TIMESTAMPTZ NOT NULL,
    channel    VARCHAR(20) NOT NULL DEFAULT 'push'
                   CHECK (channel IN ('push', 'email', 'popup')),
    sent       BOOLEAN NOT NULL DEFAULT FALSE,
    sent_at    TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_tasks_sr_remind ON tasks.scheduled_reminders(remind_at) WHERE sent = FALSE;

-- ── Triggers ──────────────────────────────────────────────────────────────────

CREATE TRIGGER boards_updated_at   BEFORE UPDATE ON tasks.boards   FOR EACH ROW EXECUTE FUNCTION tasks.set_updated_at();
CREATE TRIGGER stacks_updated_at   BEFORE UPDATE ON tasks.stacks   FOR EACH ROW EXECUTE FUNCTION tasks.set_updated_at();
CREATE TRIGGER tasks_updated_at    BEFORE UPDATE ON tasks.tasks    FOR EACH ROW EXECUTE FUNCTION tasks.set_updated_at();
CREATE TRIGGER comments_updated_at BEFORE UPDATE ON tasks.comments FOR EACH ROW EXECUTE FUNCTION tasks.set_updated_at();

CREATE TRIGGER tasks_ctag_bump AFTER INSERT OR UPDATE OR DELETE ON tasks.tasks
    FOR EACH ROW EXECUTE FUNCTION tasks.bump_board_ctag();
