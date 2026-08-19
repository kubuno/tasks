/**
 * Items of the shell's "New" button for the tasks module.
 *
 * Contributed as DATA (`MenuItem[]` from @ui) to the `shell.new-actions`
 * extension point — the shell renders them with the project's MenuDropdown.
 * `newActionItems` is evaluated when the menu OPENS, outside any React
 * component: no hooks here (store via `getState()`, i18n via `i18n.t`,
 * react-query through the captured host client in `queryClient.ts`).
 */
import { Columns3, CheckSquare } from 'lucide-react'
import { i18n, prompt, navigate } from '@kubuno/sdk'
import type { MenuItem } from '@ui'
import { tasksApi } from './api'
import { useTasksStore } from './store'
import { invalidateQueries } from './queryClient'

async function createBoard(): Promise<void> {
  const t = (key: string) => i18n.t(`tasks:${key}`)
  const title = await prompt({ title: t('new_board'), placeholder: t('title'), confirmLabel: t('create') })
  if (!title?.trim()) return
  const board = await tasksApi.createBoard({ title: title.trim() })
  invalidateQueries(['tasks-boards'])
  navigate(`/tasks/boards/${board.id}`)
}

async function createTask(): Promise<void> {
  const t = (key: string) => i18n.t(`tasks:${key}`)
  const title = await prompt({ title: t('new_task'), placeholder: t('title'), confirmLabel: t('create') })
  if (!title?.trim()) return

  // Target board: the current one, else the user's DEFAULT board (the backend
  // always guarantees one through listBoards).
  let boardId = useTasksStore.getState().currentBoardId
  if (!boardId) {
    const boards = await tasksApi.listBoards()
    boardId = (boards.find(b => b.is_default) ?? boards.find(b => !b.is_archived))?.id ?? null
    if (!boardId) {
      const board = await tasksApi.createBoard({ title: t('tasks') })
      boardId = board.id
    }
    invalidateQueries(['tasks-boards'])
  }

  await tasksApi.createTask({ board_id: boardId, title: title.trim() })
  invalidateQueries(['tasks-board', boardId], ['tasks-list'])
  navigate(`/tasks/boards/${boardId}`)
}

export function newActionItems(): MenuItem[] {
  if (!window.location.pathname.startsWith('/tasks')) return []

  return [
    {
      type: 'action',
      label: i18n.t('tasks:new_board'),
      icon: <Columns3 size={16} />,
      onClick: () => { void createBoard() },
    },
    {
      type: 'action',
      label: i18n.t('tasks:new_task'),
      icon: <CheckSquare size={16} />,
      onClick: () => { void createTask() },
    },
  ]
}
