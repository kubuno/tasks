import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { CheckSquare } from 'lucide-react'
import { DashboardWidget } from '@kubuno/sdk'
import { tasksApi } from './api'
import { isOverdue, shortDateTime, priorityLevel, PRIORITY_COLORS } from './helpers'

export default function TasksDueWidget() {
  const { t } = useTranslation('tasks')

  const { data: upcoming = [] } = useQuery({
    queryKey: ['widget-tasks-upcoming'],
    queryFn: () => tasksApi.listTasks({ collection: 'upcoming' }),
  })
  const { data: overdue = [] } = useQuery({
    queryKey: ['widget-tasks-overdue'],
    queryFn: () => tasksApi.listTasks({ collection: 'overdue' }),
  })

  const items = [...overdue, ...upcoming].slice(0, 7)

  return (
    <DashboardWidget
      title={t('tasks')}
      icon={<CheckSquare size={16} className="text-primary" />}
      link="/tasks"
      linkLabel={t('see_all')}
    >
      {items.length === 0 ? (
        <p className="text-center text-text-tertiary text-sm py-8">{t('no_upcoming_tasks')}</p>
      ) : (
        <ul className="divide-y divide-border">
          {items.map(tk => (
            <li key={tk.id} className="flex items-center gap-2 px-4 py-2.5">
              <span className="w-1.5 h-1.5 rounded-full flex-shrink-0" style={{ backgroundColor: PRIORITY_COLORS[priorityLevel(tk.priority)] }} />
              <span className="flex-1 text-sm text-text-primary truncate">{tk.title}</span>
              {tk.due_at && (
                <span className={`text-xs ${isOverdue(tk.due_at, tk.status) ? 'text-danger font-medium' : 'text-text-tertiary'}`}>
                  {shortDateTime(tk.due_at, tk.all_day)}
                </span>
              )}
            </li>
          ))}
        </ul>
      )}
    </DashboardWidget>
  )
}
