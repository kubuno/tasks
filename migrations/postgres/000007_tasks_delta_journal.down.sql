-- Restore the sequence + trigger delta machinery of 000005 and drop the journal
-- counter. (The tombstone tables were never dropped, so they are reused as-is.)

DROP TABLE IF EXISTS tasks.change_counter;

CREATE SEQUENCE IF NOT EXISTS tasks.board_change_seq;
ALTER TABLE tasks.boards ALTER COLUMN change_seq SET DEFAULT nextval('tasks.board_change_seq');

CREATE OR REPLACE FUNCTION tasks.bump_board_change_seq() RETURNS trigger AS $$
BEGIN
    NEW.change_seq := nextval('tasks.board_change_seq');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER trg_boards_change_seq BEFORE UPDATE ON tasks.boards
    FOR EACH ROW EXECUTE FUNCTION tasks.bump_board_change_seq();

CREATE OR REPLACE FUNCTION tasks.board_tombstone() RETURNS trigger AS $$
BEGIN
    INSERT INTO tasks.board_tombstones (id, owner_id, change_seq)
    VALUES (OLD.id, OLD.owner_id, nextval('tasks.board_change_seq'))
    ON CONFLICT (id) DO UPDATE SET change_seq = EXCLUDED.change_seq, deleted_at = NOW();
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER trg_boards_tombstone AFTER DELETE ON tasks.boards
    FOR EACH ROW EXECUTE FUNCTION tasks.board_tombstone();

CREATE OR REPLACE FUNCTION tasks.child_bump_board() RETURNS trigger AS $$
BEGIN
    UPDATE tasks.boards SET change_seq = change_seq
     WHERE id = COALESCE(NEW.board_id, OLD.board_id);
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER trg_stacks_bump_board AFTER INSERT OR UPDATE OR DELETE ON tasks.stacks
    FOR EACH ROW EXECUTE FUNCTION tasks.child_bump_board();
CREATE TRIGGER trg_labels_bump_board AFTER INSERT OR UPDATE OR DELETE ON tasks.labels
    FOR EACH ROW EXECUTE FUNCTION tasks.child_bump_board();
CREATE TRIGGER trg_bcomments_bump_board AFTER INSERT OR UPDATE OR DELETE ON tasks.board_comments
    FOR EACH ROW EXECUTE FUNCTION tasks.child_bump_board();

CREATE SEQUENCE IF NOT EXISTS tasks.task_change_seq;
ALTER TABLE tasks.tasks ALTER COLUMN change_seq SET DEFAULT nextval('tasks.task_change_seq');

CREATE OR REPLACE FUNCTION tasks.bump_task_change_seq() RETURNS trigger AS $$
BEGIN
    NEW.change_seq := nextval('tasks.task_change_seq');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER trg_tasks_change_seq BEFORE UPDATE ON tasks.tasks
    FOR EACH ROW EXECUTE FUNCTION tasks.bump_task_change_seq();

CREATE OR REPLACE FUNCTION tasks.task_tombstone() RETURNS trigger AS $$
BEGIN
    INSERT INTO tasks.task_tombstones (id, owner_id, change_seq)
    VALUES (OLD.id, OLD.owner_id, nextval('tasks.task_change_seq'))
    ON CONFLICT (id) DO UPDATE SET change_seq = EXCLUDED.change_seq, deleted_at = NOW();
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER trg_tasks_tombstone AFTER DELETE ON tasks.tasks
    FOR EACH ROW EXECUTE FUNCTION tasks.task_tombstone();

CREATE OR REPLACE FUNCTION tasks.child_bump_task() RETURNS trigger AS $$
BEGIN
    UPDATE tasks.tasks SET change_seq = change_seq
     WHERE id = COALESCE(NEW.task_id, OLD.task_id);
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;
CREATE TRIGGER trg_comments_bump_task AFTER INSERT OR UPDATE OR DELETE ON tasks.comments
    FOR EACH ROW EXECUTE FUNCTION tasks.child_bump_task();
CREATE TRIGGER trg_tlabels_bump_task AFTER INSERT OR DELETE ON tasks.task_labels
    FOR EACH ROW EXECUTE FUNCTION tasks.child_bump_task();
CREATE TRIGGER trg_tassignees_bump_task AFTER INSERT OR DELETE ON tasks.task_assignees
    FOR EACH ROW EXECUTE FUNCTION tasks.child_bump_task();
