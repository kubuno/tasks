import { useTranslation } from 'react-i18next'
import { Calendar, CheckCircle2, Circle, MessageSquare, ListTree } from 'lucide-react'
import { Checkbox } from '@ui'
import type { Task } from './api'
import { priorityLevel, PRIORITY_COLORS, isOverdue, shortDateTime } from './helpers'

interface Props {
  task: Task
  onOpen: (id: string) => void
  onToggleComplete: (task: Task) => void
  /** Couleur effective de la carte (couleur de la tâche, sinon du board). */
  accent?: string
  draggable?: boolean
  onDragStart?: (e: React.DragEvent, task: Task) => void
  onContextMenu?: (e: React.MouseEvent, task: Task) => void
  selected?: boolean
  onToggleSelect?: (task: Task) => void
  selectionActive?: boolean
}

export default function TaskCard({
  task, onOpen, onToggleComplete, accent, draggable, onDragStart,
  onContextMenu, selected, onToggleSelect, selectionActive,
}: Props) {
  const { t } = useTranslation('tasks')
  const level = priorityLevel(task.priority)
  const done = task.status === 'done'
  const overdue = isOverdue(task.due_at, task.status)
  const borderColor = accent ?? task.color ?? PRIORITY_COLORS[level]

  return (
    <div
      draggable={draggable}
      onDragStart={(e) => onDragStart?.(e, task)}
      onClick={(e) => { if (selectionActive && onToggleSelect) { onToggleSelect(task) } else { onOpen(task.id) } void e }}
      onContextMenu={(e) => onContextMenu?.(e, task)}
      className={`group rounded-lg border p-2.5 cursor-pointer hover:shadow-sm transition-shadow ${
        draggable ? 'active:cursor-grabbing' : ''
      } ${selected ? 'border-primary ring-1 ring-primary' : 'border-border'}`}
      style={{ background: `color-mix(in srgb, ${borderColor} 9%, white)` }}
    >
      <div className="flex items-start gap-2">
        {onToggleSelect && (selectionActive || selected) ? (
          <span className="mt-0.5 flex-shrink-0" onClick={(e) => e.stopPropagation()}>
            <Checkbox
              checked={!!selected}
              onChange={() => onToggleSelect(task)}
            />
          </span>
        ) : (
          <button
            onClick={(e) => { e.stopPropagation(); onToggleComplete(task) }}
            className="mt-0.5 text-text-tertiary hover:text-success flex-shrink-0"
            title={done ? t('reopen') : t('mark_done')}
          >
            {done ? <CheckCircle2 size={16} className="text-success" /> : <Circle size={16} />}
          </button>
        )}
        <div className="min-w-0 flex-1">
          <p className={`text-sm leading-snug break-words ${done ? 'line-through text-text-tertiary' : 'text-text-primary'}`}>
            {level !== 'none' && (
              <span
                className="inline-block w-1.5 h-1.5 rounded-full mr-1.5 align-middle"
                style={{ backgroundColor: PRIORITY_COLORS[level] }}
                title={t(`priority_${level}`)}
              />
            )}
            {task.title}
          </p>

          {(task.labels && task.labels.length > 0) && (
            <div className="flex flex-wrap gap-1 mt-1.5">
              {task.labels.map(l => (
                <span key={l.id} className="text-[10px] px-1.5 py-0.5 rounded-full text-white" style={{ backgroundColor: l.color }}>
                  {l.title}
                </span>
              ))}
            </div>
          )}

          <div className="flex items-center gap-3 mt-1.5 text-[11px] text-text-tertiary">
            {task.due_at && (
              <span className={`flex items-center gap-1 ${overdue ? 'text-danger font-medium' : ''}`}>
                <Calendar size={11} />
                {shortDateTime(task.due_at, task.all_day)}
              </span>
            )}
            {!!task.subtask_count && (
              <span className="flex items-center gap-1"><ListTree size={11} />{task.subtask_count}</span>
            )}
            {!!task.comment_count && (
              <span className="flex items-center gap-1"><MessageSquare size={11} />{task.comment_count}</span>
            )}
            {task.percent_complete > 0 && !done && (
              <span>{task.percent_complete}%</span>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
