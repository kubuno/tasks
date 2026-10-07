-- Commentaires au niveau d'un board (lisibles/écrits par tout utilisateur ayant accès au board).
CREATE TABLE IF NOT EXISTS tasks.board_comments (
    id         UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    board_id   UUID NOT NULL REFERENCES tasks.boards(id) ON DELETE CASCADE,
    author_id  UUID NOT NULL,
    body       TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_tasks_bc_comments_board ON tasks.board_comments(board_id, created_at);

CREATE TRIGGER board_comments_updated_at BEFORE UPDATE ON tasks.board_comments
    FOR EACH ROW EXECUTE FUNCTION tasks.set_updated_at();
