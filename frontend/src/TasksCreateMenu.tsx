import * as DropdownMenu from '@radix-ui/react-dropdown-menu'
import { Columns3, CheckSquare } from 'lucide-react'
import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import { prompt } from '@kubuno/sdk'
import { tasksApi } from './api'
import { useTasksStore } from './store'

const ITEM_CLASS =
  'flex items-center gap-3 w-full px-3 py-2 text-sm text-text-primary ' +
  'hover:bg-surface-1 cursor-pointer outline-none'

export default function TasksCreateMenu() {
  const navigate = useNavigate()
  const { t } = useTranslation('tasks')
  const qc = useQueryClient()

  const createBoard = async () => {
    const title = await prompt({ title: t('new_board'), placeholder: t('title'), confirmLabel: t('create') })
    if (!title?.trim()) return
    const board = await tasksApi.createBoard({ title: title.trim() })
    qc.invalidateQueries({ queryKey: ['tasks-boards'] })
    navigate(`/tasks/boards/${board.id}`)
  }

  const createTask = async () => {
    const title = await prompt({ title: t('new_task'), placeholder: t('title'), confirmLabel: t('create') })
    if (!title?.trim()) return

    // Board cible : le board courant, sinon le board PAR DÉFAUT de l'utilisateur
    // (le backend en garantit toujours un via listBoards).
    let boardId = useTasksStore.getState().currentBoardId
    if (!boardId) {
      const boards = await tasksApi.listBoards()
      boardId = (boards.find(b => b.is_default) ?? boards.find(b => !b.is_archived))?.id ?? null
      if (!boardId) {
        const board = await tasksApi.createBoard({ title: t('tasks') })
        boardId = board.id
      }
      qc.invalidateQueries({ queryKey: ['tasks-boards'] })
    }

    await tasksApi.createTask({ board_id: boardId, title: title.trim() })
    qc.invalidateQueries({ queryKey: ['tasks-board', boardId] })
    qc.invalidateQueries({ queryKey: ['tasks-list'] })
    navigate(`/tasks/boards/${boardId}`)
  }

  return (
    <>
      <DropdownMenu.Item onSelect={createBoard} className={ITEM_CLASS}>
        <Columns3 size={16} className="text-text-secondary" />
        {t('new_board')}
      </DropdownMenu.Item>
      <DropdownMenu.Item onSelect={createTask} className={ITEM_CLASS}>
        <CheckSquare size={16} className="text-text-secondary" />
        {t('new_task')}
      </DropdownMenu.Item>
    </>
  )
}
