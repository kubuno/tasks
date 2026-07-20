import type { MenuItem } from '@ui'
import type { Task, Board } from './api'

type TFn = (k: string, opts?: Record<string, unknown>) => string

export interface TaskMenuActions {
  onOpen: (id: string) => void
  onToggleDone: (task: Task) => void
  onSetPriority: (id: string, priority: number) => void
  onMoveToBoard: (id: string, boardId: string) => void
  onAddSubtask: (task: Task) => void
  onExportIcs: (task: Task) => void
  onCopyCard: (task: Task) => void
  onKubunoLabels: (task: Task) => void
  onDelete: (task: Task) => void
}

/** Construit les items du menu contextuel d'une tâche (carte Kanban ou ligne Liste). */
export function buildTaskMenu(
  task: Task,
  boards: Board[],
  t: TFn,
  a: TaskMenuActions,
): MenuItem[] {
  const done = task.status === 'done'
  const otherBoards = boards.filter(b => b.id !== task.board_id && !b.is_archived)

  const items: MenuItem[] = [
    { type: 'action', label: t('details'), onClick: () => a.onOpen(task.id) },
    { type: 'action', label: done ? t('reopen') : t('mark_done'), onClick: () => a.onToggleDone(task) },
    {
      type: 'submenu', label: t('priority'), items: [
        { type: 'action', label: t('priority_high'),   checked: task.priority >= 1 && task.priority <= 4, onClick: () => a.onSetPriority(task.id, 1) },
        { type: 'action', label: t('priority_medium'), checked: task.priority === 5,                       onClick: () => a.onSetPriority(task.id, 5) },
        { type: 'action', label: t('priority_low'),    checked: task.priority >= 6 && task.priority <= 9, onClick: () => a.onSetPriority(task.id, 9) },
        { type: 'action', label: t('priority_none'),   checked: task.priority === 0,                       onClick: () => a.onSetPriority(task.id, 0) },
      ],
    },
    { type: 'action', label: t('add_subtask'), onClick: () => a.onAddSubtask(task) },
  ]

  if (otherBoards.length > 0) {
    items.push({
      type: 'submenu',
      label: t('move_to_board'),
      items: otherBoards.map(b => ({
        type: 'action' as const,
        label: b.is_default ? t('default_board') : b.title,
        onClick: () => a.onMoveToBoard(task.id, b.id),
      })),
    })
  }

  items.push(
    { type: 'action', label: t('export_ics'), onClick: () => a.onExportIcs(task) },
    // Cross-module copy: JSON envelope pasteable as a rich card in chat, notes…
    { type: 'action', label: t('copy_card', { defaultValue: 'Copier pour Kubuno' }), onClick: () => a.onCopyCard(task) },
    // Cross-module labels (core-managed, browsable at /labels).
    { type: 'action', label: t('kubuno_labels', { defaultValue: 'Étiquettes Kubuno…' }), onClick: () => a.onKubunoLabels(task) },
    { type: 'separator' },
    { type: 'action', label: t('delete'), icon: undefined, onClick: () => a.onDelete(task) },
  )
  return items
}
