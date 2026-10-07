import { useTranslation } from 'react-i18next'
import { CalendarDays, Repeat, Star, ListTodo } from 'lucide-react'
import { addDays, formatDate, isSameDay, isToday, toDate } from '@kubuno/sdk'
import type { Task } from './api'
import { TaskCircle } from './TaskIcons'
import { isOverdue } from './helpers'

interface Props {
  task: Task
  /** Nesting level — a subtask is drawn indented under its parent. */
  depth?: number
  /** Name of the list this task belongs to, shown as a chip in cross-list views. */
  listName?: string
  onOpen: (id: string) => void
  onToggleDone: (task: Task) => void
  onToggleStar: (task: Task) => void
  onContextMenu?: (e: React.MouseEvent, task: Task) => void
  /** Drag and drop within a list (hand-made ordering). */
  draggable?: boolean
  onDragStart?: (e: React.DragEvent, task: Task) => void
  onDragOver?: (e: React.DragEvent, task: Task) => void
  onDrop?: (e: React.DragEvent, task: Task) => void
  dropBefore?: boolean
}

/** The SDK has `isToday` but no `isTomorrow`; one day out is the same question. */
function isTomorrow(d: Date): boolean {
  return isSameDay(d, addDays(new Date(), 1))
}

/** A bordered pill, the shape every piece of task metadata takes. */
function Chip({ tone = 'neutral', children }: { tone?: 'neutral' | 'accent' | 'danger'; children: React.ReactNode }) {
  const skin =
    tone === 'danger' ? 'border-danger/40 text-danger'
    : tone === 'accent' ? 'border-primary/40 text-primary'
    : 'border-border text-text-secondary'
  return (
    <span className={`inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-xs leading-4 ${skin}`}>
      {children}
    </span>
  )
}

/** The due date as a short human phrase — "today" reads better than a date. */
function dueLabel(task: Task, t: (k: string) => string): string {
  if (!task.due_at) return ''
  const d = toDate(task.due_at)
  const day = isToday(d) ? t('chip_today') : isTomorrow(d) ? t('chip_tomorrow') : formatDate(d, 'dateShort')
  return task.all_day ? day : `${day}, ${formatDate(d, 'time')}`
}

/**
 * One task inside a list column.
 *
 * The whole row opens the task; the circle and the star are separate hit
 * targets that must not fall through to it, hence the stopped propagation.
 */
export default function TaskLine({
  task, depth = 0, listName, onOpen, onToggleDone, onToggleStar, onContextMenu,
  draggable, onDragStart, onDragOver, onDrop, dropBefore,
}: Props) {
  const { t } = useTranslation('tasks')
  const done = task.status === 'done'
  const overdue = isOverdue(task.due_at, task.status)
  const due = dueLabel(task, t)
  const soon = !overdue && task.due_at && (isToday(toDate(task.due_at)) || isTomorrow(toDate(task.due_at)))

  return (
    <div
      draggable={draggable}
      onDragStart={(e) => onDragStart?.(e, task)}
      onDragOver={(e) => onDragOver?.(e, task)}
      onDrop={(e) => onDrop?.(e, task)}
      onClick={() => onOpen(task.id)}
      onContextMenu={(e) => onContextMenu?.(e, task)}
      className={`group/task relative flex items-start gap-3 rounded-lg py-2 pr-2 cursor-pointer
                  hover:bg-surface-1 ${dropBefore ? 'shadow-[inset_0_2px_0_0_var(--color-primary)]' : ''}`}
      style={{ paddingLeft: 8 + depth * 26 }}
    >
      <button
        type="button"
        onClick={(e) => { e.stopPropagation(); onToggleDone(task) }}
        title={done ? t('reopen') : t('mark_done')}
        aria-label={done ? t('reopen') : t('mark_done')}
        className={`mt-px flex-shrink-0 rounded-full outline-none focus-visible:ring-2 focus-visible:ring-primary
                    ${done ? 'text-primary' : 'text-text-secondary hover:text-primary'}`}
      >
        <TaskCircle done={done} />
      </button>

      <div className="min-w-0 flex-1">
        <div className={`text-sm leading-5 break-words ${done ? 'line-through text-text-tertiary' : 'text-text-nav'}`}>
          {task.title}
        </div>
        {task.description && (
          <div className="mt-0.5 text-xs leading-4 text-text-secondary line-clamp-2 break-words">
            {task.description}
          </div>
        )}
        {(due || listName) && (
          <div className="mt-1.5 flex flex-wrap items-center gap-1.5">
            {due && (
              <Chip tone={overdue ? 'danger' : soon ? 'accent' : 'neutral'}>
                <CalendarDays size={12} aria-hidden="true" />
                {due}
                {task.rrule && <Repeat size={11} aria-hidden="true" />}
              </Chip>
            )}
            {listName && (
              <Chip>
                <ListTodo size={12} aria-hidden="true" />
                {listName}
              </Chip>
            )}
          </div>
        )}
      </div>

      <button
        type="button"
        onClick={(e) => { e.stopPropagation(); onToggleStar(task) }}
        title={task.starred ? t('unstar') : t('star')}
        aria-label={task.starred ? t('unstar') : t('star')}
        aria-pressed={task.starred}
        className={`mt-0.5 flex-shrink-0 rounded p-0.5 outline-none focus-visible:ring-2 focus-visible:ring-primary
                    ${task.starred
                      ? 'text-primary'
                      : 'text-text-tertiary opacity-0 group-hover/task:opacity-100 focus-visible:opacity-100 hover:text-text-primary'}`}
      >
        <Star size={17} fill={task.starred ? 'currentColor' : 'none'} />
      </button>
    </div>
  )
}
