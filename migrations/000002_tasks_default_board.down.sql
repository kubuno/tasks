DROP INDEX IF EXISTS tasks.idx_tasks_board_one_default;
ALTER TABLE tasks.boards DROP COLUMN IF EXISTS is_default;
