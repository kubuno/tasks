-- Board par défaut, non supprimable et non renommable, garanti par utilisateur.
ALTER TABLE tasks.boards ADD COLUMN IF NOT EXISTS is_default BOOLEAN NOT NULL DEFAULT FALSE;

-- Au plus un board par défaut par propriétaire.
CREATE UNIQUE INDEX IF NOT EXISTS idx_tasks_board_one_default
    ON tasks.boards(owner_id) WHERE is_default;
