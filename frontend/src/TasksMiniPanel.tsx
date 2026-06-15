import { useQuery } from '@tanstack/react-query'
import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { CheckSquare } from 'lucide-react'
import { Spinner } from '@ui'
import { tasksApi } from './api'
import { isOverdue, shortDateTime, priorityLevel, PRIORITY_COLORS } from './helpers'

export default function TasksMiniPanel() {
  const { t } = useTranslation('tasks')
  const navigate = useNavigate()

  const { data: tasks = [], isLoading } = useQuery({
    queryKey: ['tasks-mini-today'],
    queryFn: () => tasksApi.listTasks({ collection: 'today' }),
  })
  const { data: overdue = [] } = useQuery({
    queryKey: ['tasks-mini-overdue'],
    queryFn: () => tasksApi.listTasks({ collection: 'overdue' }),
  })

  return (
    <div className="p-3">
      <div className="flex items-center gap-2 mb-3">
        <CheckSquare size={18} className="text-primary" />
        <h2 className="text-sm font-semibold text-text-primary">{t('tasks')}</h2>
      </div>

      {isLoading ? (
        <div className="flex justify-center py-6"><Spinner /></div>
      ) : (
        <div className="space-y-4">
          {overdue.length > 0 && (
            <Section title={t('collection_overdue')} count={overdue.length}>
              {overdue.slice(0, 5).map(tk => (
                <Row key={tk.id} title={tk.title} due={shortDateTime(tk.due_at, tk.all_day)} danger color={PRIORITY_COLORS[priorityLevel(tk.priority)]} onClick={() => navigate(`/tasks/boards/${tk.board_id}`)} />
              ))}
            </Section>
          )}
          <Section title={t('collection_today')} count={tasks.length}>
            {tasks.length === 0 ? (
              <p className="text-xs text-text-tertiary">{t('no_tasks')}</p>
            ) : tasks.slice(0, 8).map(tk => (
              <Row key={tk.id} title={tk.title} due={shortDateTime(tk.due_at, tk.all_day)} danger={isOverdue(tk.due_at, tk.status)} color={PRIORITY_COLORS[priorityLevel(tk.priority)]} onClick={() => navigate(`/tasks/boards/${tk.board_id}`)} />
            ))}
          </Section>
        </div>
      )}
    </div>
  )
}

function Section({ title, count, children }: { title: string; count: number; children: React.ReactNode }) {
  return (
    <div>
      <div className="text-[11px] font-semibold uppercase tracking-wide text-text-tertiary mb-1.5">{title} · {count}</div>
      <div className="space-y-1">{children}</div>
    </div>
  )
}

function Row({ title, due, danger, color, onClick }: { title: string; due: string; danger?: boolean; color: string; onClick: () => void }) {
  return (
    <button onClick={onClick} className="flex items-center gap-2 w-full text-left px-2 py-1.5 rounded-lg hover:bg-surface-1">
      <span className="w-1.5 h-1.5 rounded-full flex-shrink-0" style={{ backgroundColor: color }} />
      <span className="flex-1 text-sm text-text-primary truncate">{title}</span>
      {due && <span className={`text-[11px] flex-shrink-0 ${danger ? 'text-danger' : 'text-text-tertiary'}`}>{due}</span>}
    </button>
  )
}
