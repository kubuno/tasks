-- A task can be "starred": flagged by its owner so it shows up in the Starred
-- view, independently of its priority (which maps to the iCalendar PRIORITY
-- property and carries a different meaning).
--
-- `starred_at` records WHEN the flag was raised so the lists can offer a
-- "recently starred" ordering; it is cleared with the flag.
ALTER TABLE tasks.tasks
    ADD COLUMN starred    boolean     NOT NULL DEFAULT false,
    ADD COLUMN starred_at timestamptz;

-- Partial index: only starred rows are ever scanned by the Starred view.
CREATE INDEX idx_tasks_starred
    ON tasks.tasks (owner_id, starred_at DESC)
    WHERE starred;
