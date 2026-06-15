-- Couleur optionnelle par tâche. NULL = hérite de la couleur du board.
ALTER TABLE tasks.tasks ADD COLUMN IF NOT EXISTS color VARCHAR(7);
