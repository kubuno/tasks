-- Move the delta layer off PostgreSQL sequences + triggers and onto the
-- application-driven `kubuno_db::journal` primitive (one shared counter row per
-- domain, seqs taken in Rust at write time, tombstones written in the same
-- transaction). Neither the sequence nor the trigger mechanism has a portable
-- form on MySQL/SQLite, so it is retired here on PostgreSQL too; the tombstone
-- TABLES keep their exact shape (no data migration), only their triggers go.
--
-- The `change_seq` columns stay `BIGINT NOT NULL`, but their DEFAULT switches
-- from `nextval(...)` to `0`: the application now supplies every value. The
-- `bump_board_ctag` trigger from 000001 is deliberately left in place — it keeps
-- refreshing the CalDAV ctag; it simply no longer bumps `change_seq` (that
-- trigger is gone), which now moves only through the journal.

-- ── Drop the trigger/function/sequence machinery from 000005 ──────────────────

-- Boards: BEFORE UPDATE seq, AFTER DELETE tombstone, child bumps.
DROP TRIGGER IF EXISTS trg_boards_change_seq   ON tasks.boards;
DROP TRIGGER IF EXISTS trg_boards_tombstone    ON tasks.boards;
DROP TRIGGER IF EXISTS trg_stacks_bump_board   ON tasks.stacks;
DROP TRIGGER IF EXISTS trg_labels_bump_board   ON tasks.labels;
DROP TRIGGER IF EXISTS trg_bcomments_bump_board ON tasks.board_comments;
DROP FUNCTION IF EXISTS tasks.bump_board_change_seq();
DROP FUNCTION IF EXISTS tasks.board_tombstone();
DROP FUNCTION IF EXISTS tasks.child_bump_board();

-- Tasks: BEFORE UPDATE seq, AFTER DELETE tombstone, child bumps.
DROP TRIGGER IF EXISTS trg_tasks_change_seq    ON tasks.tasks;
DROP TRIGGER IF EXISTS trg_tasks_tombstone     ON tasks.tasks;
DROP TRIGGER IF EXISTS trg_comments_bump_task  ON tasks.comments;
DROP TRIGGER IF EXISTS trg_tlabels_bump_task   ON tasks.task_labels;
DROP TRIGGER IF EXISTS trg_tassignees_bump_task ON tasks.task_assignees;
DROP FUNCTION IF EXISTS tasks.bump_task_change_seq();
DROP FUNCTION IF EXISTS tasks.task_tombstone();
DROP FUNCTION IF EXISTS tasks.child_bump_task();

-- The DEFAULT references the sequence, so it must go before the sequence does.
ALTER TABLE tasks.boards ALTER COLUMN change_seq SET DEFAULT 0;
ALTER TABLE tasks.tasks  ALTER COLUMN change_seq SET DEFAULT 0;
DROP SEQUENCE IF EXISTS tasks.board_change_seq;
DROP SEQUENCE IF EXISTS tasks.task_change_seq;

-- ── The journal's shared counter, seeded to continue the existing sequences ───

CREATE TABLE IF NOT EXISTS tasks.change_counter (
    domain VARCHAR(190) NOT NULL PRIMARY KEY,
    n      BIGINT       NOT NULL
);

-- Seed each domain to the current max so `next_seq` (n := n + 1) never hands out
-- a value an existing row already holds.
INSERT INTO tasks.change_counter (domain, n)
    SELECT 'boards', COALESCE(MAX(change_seq), 0) FROM tasks.boards
    ON CONFLICT (domain) DO NOTHING;
INSERT INTO tasks.change_counter (domain, n)
    SELECT 'tasks', COALESCE(MAX(change_seq), 0) FROM tasks.tasks
    ON CONFLICT (domain) DO NOTHING;

-- The tombstone tables (tasks.board_tombstones, tasks.task_tombstones) keep
-- their 000005 shape unchanged; only their triggers were dropped above.
