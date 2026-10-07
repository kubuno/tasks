DROP INDEX IF EXISTS tasks.idx_tasks_linked_files;
ALTER TABLE tasks.tasks
    ALTER COLUMN linked_file_ids DROP DEFAULT,
    ALTER COLUMN linked_file_ids TYPE uuid[]
        USING (SELECT COALESCE(array_agg(e::uuid), '{}')
                 FROM jsonb_array_elements_text(linked_file_ids) AS e),
    ALTER COLUMN linked_file_ids SET DEFAULT '{}';
