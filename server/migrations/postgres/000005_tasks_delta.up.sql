-- Delta primitives for the tasks local-first pull (boards, tasks): monotonic
-- change_seq + tombstones. Children bump their sync parent: stacks/labels/
-- board_comments -> board ; comments/task_labels/task_assignees -> task.
-- NB: tasks already has bump_board_ctag (AFTER I/U/D ON tasks -> UPDATE boards),
-- so every task mutation also bumps its board's change_seq via the BEFORE UPDATE
-- trigger below — no extra task->board trigger needed.

CREATE SEQUENCE IF NOT EXISTS tasks.board_change_seq;
ALTER TABLE tasks.boards ADD COLUMN IF NOT EXISTS change_seq BIGINT NOT NULL DEFAULT nextval('tasks.board_change_seq');
CREATE INDEX IF NOT EXISTS idx_tasks_boards_change_seq ON tasks.boards(owner_id, change_seq);

CREATE OR REPLACE FUNCTION tasks.bump_board_change_seq() RETURNS trigger AS $$
BEGIN
    NEW.change_seq := nextval('tasks.board_change_seq');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_boards_change_seq ON tasks.boards;
CREATE TRIGGER trg_boards_change_seq BEFORE UPDATE ON tasks.boards
    FOR EACH ROW EXECUTE FUNCTION tasks.bump_board_change_seq();

CREATE TABLE IF NOT EXISTS tasks.board_tombstones (
    id         UUID        PRIMARY KEY,
    owner_id   UUID        NOT NULL,
    change_seq BIGINT      NOT NULL,
    deleted_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_tasks_board_tomb_seq ON tasks.board_tombstones(owner_id, change_seq);

CREATE OR REPLACE FUNCTION tasks.board_tombstone() RETURNS trigger AS $$
BEGIN
    INSERT INTO tasks.board_tombstones (id, owner_id, change_seq)
    VALUES (OLD.id, OLD.owner_id, nextval('tasks.board_change_seq'))
    ON CONFLICT (id) DO UPDATE SET change_seq = EXCLUDED.change_seq, deleted_at = NOW();
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_boards_tombstone ON tasks.boards;
CREATE TRIGGER trg_boards_tombstone AFTER DELETE ON tasks.boards
    FOR EACH ROW EXECUTE FUNCTION tasks.board_tombstone();

CREATE OR REPLACE FUNCTION tasks.child_bump_board() RETURNS trigger AS $$
BEGIN
    UPDATE tasks.boards SET change_seq = change_seq
     WHERE id = COALESCE(NEW.board_id, OLD.board_id);
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_stacks_bump_board ON tasks.stacks;
CREATE TRIGGER trg_stacks_bump_board AFTER INSERT OR UPDATE OR DELETE ON tasks.stacks
    FOR EACH ROW EXECUTE FUNCTION tasks.child_bump_board();
DROP TRIGGER IF EXISTS trg_labels_bump_board ON tasks.labels;
CREATE TRIGGER trg_labels_bump_board AFTER INSERT OR UPDATE OR DELETE ON tasks.labels
    FOR EACH ROW EXECUTE FUNCTION tasks.child_bump_board();
DROP TRIGGER IF EXISTS trg_bcomments_bump_board ON tasks.board_comments;
CREATE TRIGGER trg_bcomments_bump_board AFTER INSERT OR UPDATE OR DELETE ON tasks.board_comments
    FOR EACH ROW EXECUTE FUNCTION tasks.child_bump_board();

CREATE SEQUENCE IF NOT EXISTS tasks.task_change_seq;
ALTER TABLE tasks.tasks ADD COLUMN IF NOT EXISTS change_seq BIGINT NOT NULL DEFAULT nextval('tasks.task_change_seq');
CREATE INDEX IF NOT EXISTS idx_tasks_tasks_change_seq ON tasks.tasks(owner_id, change_seq);

CREATE OR REPLACE FUNCTION tasks.bump_task_change_seq() RETURNS trigger AS $$
BEGIN
    NEW.change_seq := nextval('tasks.task_change_seq');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_tasks_change_seq ON tasks.tasks;
CREATE TRIGGER trg_tasks_change_seq BEFORE UPDATE ON tasks.tasks
    FOR EACH ROW EXECUTE FUNCTION tasks.bump_task_change_seq();

CREATE TABLE IF NOT EXISTS tasks.task_tombstones (
    id         UUID        PRIMARY KEY,
    owner_id   UUID        NOT NULL,
    change_seq BIGINT      NOT NULL,
    deleted_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_tasks_task_tomb_seq ON tasks.task_tombstones(owner_id, change_seq);

CREATE OR REPLACE FUNCTION tasks.task_tombstone() RETURNS trigger AS $$
BEGIN
    INSERT INTO tasks.task_tombstones (id, owner_id, change_seq)
    VALUES (OLD.id, OLD.owner_id, nextval('tasks.task_change_seq'))
    ON CONFLICT (id) DO UPDATE SET change_seq = EXCLUDED.change_seq, deleted_at = NOW();
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_tasks_tombstone ON tasks.tasks;
CREATE TRIGGER trg_tasks_tombstone AFTER DELETE ON tasks.tasks
    FOR EACH ROW EXECUTE FUNCTION tasks.task_tombstone();

CREATE OR REPLACE FUNCTION tasks.child_bump_task() RETURNS trigger AS $$
BEGIN
    UPDATE tasks.tasks SET change_seq = change_seq
     WHERE id = COALESCE(NEW.task_id, OLD.task_id);
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_comments_bump_task ON tasks.comments;
CREATE TRIGGER trg_comments_bump_task AFTER INSERT OR UPDATE OR DELETE ON tasks.comments
    FOR EACH ROW EXECUTE FUNCTION tasks.child_bump_task();
DROP TRIGGER IF EXISTS trg_tlabels_bump_task ON tasks.task_labels;
CREATE TRIGGER trg_tlabels_bump_task AFTER INSERT OR DELETE ON tasks.task_labels
    FOR EACH ROW EXECUTE FUNCTION tasks.child_bump_task();
DROP TRIGGER IF EXISTS trg_tassignees_bump_task ON tasks.task_assignees;
CREATE TRIGGER trg_tassignees_bump_task AFTER INSERT OR DELETE ON tasks.task_assignees
    FOR EACH ROW EXECUTE FUNCTION tasks.child_bump_task();
