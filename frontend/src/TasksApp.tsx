import { useEffect } from 'react'
import { useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import TasksKanbanBoard from './TasksKanbanBoard'
import TasksListView from './TasksListView'
import TaskDetailPanel from './TaskDetailPanel'
import { useTasksStore } from './store'

export default function TasksApp() {
  const { t } = useTranslation('tasks')
  const params = useParams()
  const boardId = params.id ?? null
  const view = useTasksStore(s => s.view)
  const setCurrentBoard = useTasksStore(s => s.setCurrentBoard)

  useEffect(() => {
    setCurrentBoard(boardId)
  }, [boardId, setCurrentBoard])

  return (
    <div className="h-full flex flex-col bg-surface-0">
      <div className="flex-1 min-h-0">
        {boardId ? (
          view === 'list' ? <TasksListView boardId={boardId} /> : <TasksKanbanBoard boardId={boardId} />
        ) : (
          <CollectionHeaderAndList />
        )}
      </div>
      <TaskDetailPanel />
    </div>
  )

  function CollectionHeaderAndList() {
    const collection = useTasksStore(s => s.collection)
    return (
      <div className="h-full flex flex-col">
        <div className="px-4 pt-4">
          <h1 className="text-xl font-semibold text-text-primary">{t(`collection_${collection}`)}</h1>
        </div>
        <div className="flex-1 min-h-0">
          <TasksListView />
        </div>
      </div>
    )
  }
}
