DROP TABLE IF EXISTS tasks.scheduled_reminders CASCADE;
DROP TABLE IF EXISTS tasks.attachments        CASCADE;
DROP TABLE IF EXISTS tasks.comments           CASCADE;
DROP TABLE IF EXISTS tasks.task_assignees     CASCADE;
DROP TABLE IF EXISTS tasks.task_labels        CASCADE;
DROP TABLE IF EXISTS tasks.tasks              CASCADE;
DROP TABLE IF EXISTS tasks.labels             CASCADE;
DROP TABLE IF EXISTS tasks.stacks             CASCADE;
DROP TABLE IF EXISTS tasks.board_shares       CASCADE;
DROP TABLE IF EXISTS tasks.boards             CASCADE;

DROP FUNCTION IF EXISTS tasks.bump_board_ctag();
DROP FUNCTION IF EXISTS tasks.set_updated_at();
