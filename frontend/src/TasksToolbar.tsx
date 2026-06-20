import { useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { LayoutGrid, List, Download, Upload } from 'lucide-react'
import { tasksApi } from './api'
import { useTasksStore, type TasksView } from './store'

export default function TasksToolbar() {
  const { t } = useTranslation('tasks')
  const params = useParams()
  const boardId = params.id ?? null
  const view = useTasksStore(s => s.view)
  const setView = useTasksStore(s => s.setView)

  const { data: board } = useQuery({
    queryKey: ['tasks-board-meta', boardId],
    queryFn: () => tasksApi.getBoard(boardId!).then(r => r.board),
    enabled: !!boardId,
  })

  if (!boardId) {
    return <div className="h-12 flex items-center px-4 text-sm font-medium text-text-secondary">{t('tasks')}</div>
  }

  const ViewBtn = ({ v, icon }: { v: TasksView; icon: React.ReactNode }) => (
    <button
      onClick={() => setView(v)}
      className={`flex items-center gap-1.5 px-2.5 py-1 rounded-md text-sm ${
        view === v ? 'bg-white shadow-sm text-text-primary' : 'text-text-secondary hover:text-text-primary'
      }`}
    >
      {icon}{t(`view_${v}`)}
    </button>
  )

  const importIcs = () => {
    const input = document.createElement('input')
    input.type = 'file'
    input.accept = '.ics,text/calendar'
    input.onchange = async () => {
      const file = input.files?.[0]
      if (!file || !boardId) return
      const text = await file.text()
      await tasksApi.importBoard(boardId, text)
      window.location.reload()
    }
    input.click()
  }

  return (
    <div className="h-12 flex items-center justify-between px-4 gap-3">
      <div className="flex items-center gap-2 min-w-0">
        <span className="w-2.5 h-2.5 rounded-full flex-shrink-0" style={{ backgroundColor: board?.color ?? '#1a73e8' }} />
        <span className="text-sm font-semibold text-text-primary truncate">{board?.title ?? ''}</span>
      </div>
      <div className="flex items-center gap-3 no-print">
        <div className="flex items-center gap-1 bg-surface-2 rounded-lg p-0.5">
          <ViewBtn v="kanban" icon={<LayoutGrid size={15} />} />
          <ViewBtn v="list" icon={<List size={15} />} />
        </div>
        <button onClick={importIcs} title={t('import_ics')} className="p-1.5 rounded hover:bg-surface-2 text-text-secondary"><Upload size={16} /></button>
        <a href={boardId ? tasksApi.exportBoardUrl(boardId) : '#'} title={t('export_ics')} className="p-1.5 rounded hover:bg-surface-2 text-text-secondary"><Download size={16} /></a>
      </div>
    </div>
  )
}
