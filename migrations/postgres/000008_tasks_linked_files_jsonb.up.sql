-- Portable array column: PostgreSQL's `UUID[]` has no equivalent on MySQL/SQLite,
-- so `linked_file_ids` becomes a JSON array of the UUIDs' text form — exactly
-- what `#[sqlx(json)] Vec<Uuid>` / kubuno-db's `DbValue::Json` read and write.
-- `to_jsonb(uuid[])` yields `["<uuid>", ...]`, the hyphenated spelling uuid's own
-- Serialize uses, so existing rows round-trip unchanged.
ALTER TABLE tasks.tasks
    ALTER COLUMN linked_file_ids DROP DEFAULT,
    ALTER COLUMN linked_file_ids TYPE jsonb USING to_jsonb(linked_file_ids),
    ALTER COLUMN linked_file_ids SET DEFAULT '[]'::jsonb;

-- A jsonb_path_ops GIN index serves the `@>` containment test
-- (Backend::json_array_contains) should a screen ever filter tasks by a linked
-- file. Small and `@>`-only, so jsonb_path_ops rather than the default opclass.
CREATE INDEX IF NOT EXISTS idx_tasks_linked_files
    ON tasks.tasks USING gin (linked_file_ids jsonb_path_ops);
