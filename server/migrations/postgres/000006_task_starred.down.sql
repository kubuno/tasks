DROP INDEX IF EXISTS tasks.idx_tasks_starred;
ALTER TABLE tasks.tasks
    DROP COLUMN IF EXISTS starred,
    DROP COLUMN IF EXISTS starred_at;
