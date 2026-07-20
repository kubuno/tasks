/**
 * `tasks.task` envelopes: card renderer + envelope builder for tasks copied
 * from the context menu ("Copier pour Kubuno"). Registered on `core.data-card`
 * from `entry.ts`; consumers (chat, notes…) resolve it dynamically.
 */
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router-dom'
import { CheckCircle2, Circle, CalendarClock, Flag } from 'lucide-react'
import type { Task } from './api'
import type { DataCardProps, KubunoDataEnvelope } from './kubunoData'

export interface TaskCardData {
  id: string
  board_id: string
  title: string
  status: string
  priority: number
  due_at: string | null
  all_day: boolean
  percent_complete: number
  description?: string
}

export function taskEnvelope(task: Task): KubunoDataEnvelope {
  const href = `/tasks/boards/${task.board_id}?task=${task.id}`
  const due = task.due_at ? new Date(task.due_at) : null
  const data: TaskCardData = {
    id: task.id, board_id: task.board_id, title: task.title, status: task.status,
    priority: task.priority, due_at: task.due_at, all_day: task.all_day,
    percent_complete: task.percent_complete, description: task.description ?? undefined,
  }
  return {
    kubuno: 1,
    type: 'tasks.task',
    module: 'tasks',
    title: task.title,
    text: `${task.title}${due ? ` — ${due.toLocaleDateString()}` : ''}\n${location.origin}${href}`,
    href,
    data,
  }
}

export default function TasksDataCard({ envelope }: DataCardProps) {
  const { t, i18n } = useTranslation('tasks')
  const navigate = useNavigate()
  const d = envelope.data as TaskCardData | null
  if (!d || typeof d.title !== 'string') return null
  const done = d.status === 'done'
  const high = d.priority >= 1 && d.priority <= 4
  const due = d.due_at ? new Date(d.due_at) : null

  return (
    <div
      className="w-72 max-w-full rounded-xl border border-border bg-surface-0 overflow-hidden cursor-pointer hover:border-strong transition-colors"
      onClick={() => { if (envelope.href) navigate(envelope.href) }}
      role="button"
      title={t('details', { defaultValue: 'Détails' })}
    >
      <div className="px-3 py-2.5 flex items-start gap-2">
        {done
          ? <CheckCircle2 size={16} className="text-success mt-0.5 flex-shrink-0" />
          : <Circle size={16} className="text-text-tertiary mt-0.5 flex-shrink-0" />}
        <div className="min-w-0 flex-1">
          <p className={`text-xs font-semibold truncate ${done ? 'text-text-tertiary line-through' : 'text-text-primary'}`}>{d.title}</p>
          <div className="flex items-center gap-2 mt-0.5">
            {due && (
              <span className="text-[11px] text-text-secondary flex items-center gap-1">
                <CalendarClock size={11} /> {due.toLocaleDateString(i18n.language)}
              </span>
            )}
            {high && (
              <span className="text-[11px] text-danger flex items-center gap-0.5">
                <Flag size={11} /> {t('priority_high', { defaultValue: 'Haute' })}
              </span>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
