import { useEffect } from 'react'
import { useParams, useSearchParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import TasksKanbanBoard from './TasksKanbanBoard'
import TasksListView from './TasksListView'
import TasksOverview from './TasksOverview'
import TaskDetailPanel from './TaskDetailPanel'
import { useTasksStore } from './store'

// Which views render as the multi-column overview, and which as a flat list.
const OVERVIEW_COLLECTIONS = new Set(['all', 'starred'])

export default function TasksApp() {
  const { t } = useTranslation('tasks')
  const params = useParams()
  const boardId = params.id ?? null
  const view = useTasksStore(s => s.view)
  const collection = useTasksStore(s => s.collection)
  const setCurrentBoard = useTasksStore(s => s.setCurrentBoard)

  useEffect(() => {
    setCurrentBoard(boardId)
  }, [boardId, setCurrentBoard])

  // Deep link `?task=<id>` (used by cross-module data cards): opens the detail panel.
  const [searchParams] = useSearchParams()
  const selectTask = useTasksStore(s => s.selectTask)
  useEffect(() => {
    const tid = searchParams.get('task')
    if (tid) selectTask(tid)
  }, [searchParams, selectTask])

  return (
    <div className="flex h-full flex-col bg-surface-0">
      <div className="min-h-0 flex-1">
        {boardId ? (
          // A single list/board opened by its own route: its column or Kanban view.
          view === 'list' ? <TasksListView boardId={boardId} /> : <TasksKanbanBoard boardId={boardId} />
        ) : OVERVIEW_COLLECTIONS.has(collection) ? (
          // The landing views: lists side by side, or the starred card.
          <TasksOverview mode={collection === 'starred' ? 'starred' : 'all'} />
        ) : (
          // A date-driven filter (today, overdue…): one flat list across every list.
          <CollectionList />
        )}
      </div>
      <TaskDetailPanel />
    </div>
  )

  function CollectionList() {
    return (
      <div className="flex h-full flex-col">
        <div className="px-6 pt-5">
          <h1 className="text-lg font-normal text-text-nav">{t(`collection_${collection}`)}</h1>
        </div>
        <div className="min-h-0 flex-1">
          <TasksListView />
        </div>
      </div>
    )
  }
}
