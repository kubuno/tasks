import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { ChevronRight, ChevronDown, Plus, CheckCircle2, Circle } from 'lucide-react'
import { Spinner, MenuDropdown, type MenuItem, Input, Button } from '@ui'
import { ConfirmDialog } from '@ui'
import { useConfirm } from '@kubuno/sdk'
import { prompt } from '@kubuno/sdk'
import { tasksApi, type Task, type Collection } from './api'
import { useTasksStore } from './store'
import { priorityLevel, PRIORITY_COLORS, isOverdue, shortDateTime } from './helpers'
import { buildTaskMenu } from './taskMenu'

interface Props {
  /** Si défini, liste les tâches du board ; sinon utilise la collection du store. */
  boardId?: string
}

function TaskRow({ task, depth, onOpen, onToggle, expandable, expanded, onExpand, onContextMenu, accent }: {
  task: Task
  depth: number
  onOpen: (id: string) => void
  onToggle: (t: Task) => void
  expandable: boolean
  expanded: boolean
  onExpand: () => void
  onContextMenu?: (e: React.MouseEvent, t: Task) => void
  accent?: string
}) {
  const { t } = useTranslation('tasks')
  const done = task.status === 'done'
  const overdue = isOverdue(task.due_at, task.status)
  const level = priorityLevel(task.priority)
  const barColor = task.color ?? accent
  return (
    <div
      onClick={() => onOpen(task.id)}
      onContextMenu={(e) => onContextMenu?.(e, task)}
      className="flex items-center gap-2 px-3 py-2 hover:bg-surface-1 cursor-pointer border-b border-border/60"
      style={{ paddingLeft: 12 + depth * 22, borderLeft: barColor ? `3px solid ${barColor}` : undefined }}
    >
      {expandable ? (
        <button onClick={(e) => { e.stopPropagation(); onExpand() }} className="text-text-tertiary hover:text-text-primary">
          {expanded ? <ChevronDown size={15} /> : <ChevronRight size={15} />}
        </button>
      ) : <span className="w-[15px]" />}

      <button
        onClick={(e) => { e.stopPropagation(); onToggle(task) }}
        className="text-text-tertiary hover:text-success flex-shrink-0"
        title={done ? t('reopen') : t('mark_done')}
      >
        {done ? <CheckCircle2 size={17} className="text-success" /> : <Circle size={17} />}
      </button>

      <span className="w-1.5 h-1.5 rounded-full flex-shrink-0" style={{ backgroundColor: PRIORITY_COLORS[level] }} />

      <span className={`flex-1 text-sm truncate ${done ? 'line-through text-text-tertiary' : 'text-text-primary'}`}>
        {task.title}
      </span>

      {(task.labels && task.labels.length > 0) && (
        <div className="hidden sm:flex gap-1">
          {task.labels.slice(0, 3).map(l => (
            <span key={l.id} className="text-[10px] px-1.5 py-0.5 rounded-full text-white" style={{ backgroundColor: l.color }}>{l.title}</span>
          ))}
        </div>
      )}

      {task.due_at && (
        <span className={`text-xs flex-shrink-0 ${overdue ? 'text-danger font-medium' : 'text-text-tertiary'}`}>
          {shortDateTime(task.due_at, task.all_day)}
        </span>
      )}
    </div>
  )
}

function Subtasks({ parentId, onOpen, onToggle, onContextMenu, accent }: { parentId: string; onOpen: (id: string) => void; onToggle: (t: Task) => void; onContextMenu?: (e: React.MouseEvent, t: Task) => void; accent?: string }) {
  const { data = [] } = useQuery({ queryKey: ['tasks-subtasks', parentId], queryFn: () => tasksApi.listSubtasks(parentId) })
  return (
    <>
      {data.map(st => (
        <TaskRow key={st.id} task={st} depth={1} onOpen={onOpen} onToggle={onToggle} expandable={false} expanded={false} onExpand={() => {}} onContextMenu={onContextMenu} accent={accent} />
      ))}
    </>
  )
}

export default function TasksListView({ boardId }: Props) {
  const { t } = useTranslation('tasks')
  const qc = useQueryClient()
  const collection = useTasksStore(s => s.collection)
  const search = useTasksStore(s => s.searchQuery)
  const selectTask = useTasksStore(s => s.selectTask)
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()
  const [expanded, setExpanded] = useState<Set<string>>(new Set())
  const [adding, setAdding] = useState(false)
  const [newTitle, setNewTitle] = useState('')
  const [taskMenu, setTaskMenu] = useState<{ task: Task; pos: { top: number; left: number } } | null>(null)

  const queryArg: { board_id?: string; collection?: Collection; search?: string } =
    boardId ? { board_id: boardId, search: search || undefined } : { collection, search: search || undefined }

  const { data: tasks = [], isLoading } = useQuery({
    queryKey: ['tasks-list', boardId ?? collection, search],
    queryFn: () => tasksApi.listTasks(queryArg),
  })
  const { data: boards = [] } = useQuery({ queryKey: ['tasks-boards'], queryFn: tasksApi.listBoards })

  const invalidate = () => {
    qc.invalidateQueries({ queryKey: ['tasks-list'] })
    qc.invalidateQueries({ queryKey: ['tasks-subtasks'] })
    qc.invalidateQueries({ queryKey: ['tasks-board'] })
  }
  const toggleMut = useMutation({
    mutationFn: (task: Task) =>
      task.status === 'done' ? tasksApi.updateTask(task.id, { status: 'open' }) : tasksApi.completeTask(task.id),
    onSuccess: invalidate,
  })
  const createMut = useMutation({
    mutationFn: (title: string) => tasksApi.createTask({ board_id: boardId!, title }),
    onSuccess: () => { setNewTitle(''); setAdding(false); invalidate() },
  })
  const setPriorityMut = useMutation({
    mutationFn: (v: { id: string; priority: number }) => tasksApi.updateTask(v.id, { priority: v.priority }),
    onSuccess: invalidate,
  })
  const deleteTaskMut = useMutation({ mutationFn: (id: string) => tasksApi.deleteTask(id), onSuccess: invalidate })
  const moveToBoardMut = useMutation({
    mutationFn: (v: { id: string; boardId: string }) => tasksApi.moveTasksToBoard([v.id], v.boardId),
    onSuccess: invalidate,
  })

  const taskActions = {
    onOpen: selectTask,
    onToggleDone: (task: Task) => toggleMut.mutate(task),
    onSetPriority: (id: string, priority: number) => setPriorityMut.mutate({ id, priority }),
    onMoveToBoard: (id: string, bId: string) => moveToBoardMut.mutate({ id, boardId: bId }),
    onAddSubtask: async (task: Task) => {
      const title = await prompt({ title: t('add_subtask'), placeholder: t('title'), confirmLabel: t('add') })
      if (title?.trim()) { await tasksApi.createSubtask(task.id, { board_id: task.board_id, title: title.trim() }); invalidate() }
    },
    onExportIcs: (task: Task) => window.open(`/api/v1/tasks/tasks/${task.id}/ics`, '_blank'),
    onDelete: async (task: Task) => {
      if (await confirm({ title: t('delete_task'), message: t('confirm_delete_task'), confirmLabel: t('delete'), variant: 'danger' }))
        deleteTaskMut.mutate(task.id)
    },
  }
  const openTaskMenu = (e: React.MouseEvent, task: Task) => { e.preventDefault(); setTaskMenu({ task, pos: { top: e.clientY, left: e.clientX } }) }
  const taskMenuItems: MenuItem[] = taskMenu ? buildTaskMenu(taskMenu.task, boards, t, taskActions) : []

  if (isLoading) return <div className="flex items-center justify-center h-full"><Spinner /></div>

  const toggleExpand = (id: string) =>
    setExpanded(prev => {
      const n = new Set(prev)
      if (n.has(id)) n.delete(id); else n.add(id)
      return n
    })

  return (
    <div className="h-full overflow-y-auto">
      <div className="max-w-3xl mx-auto py-4">
        {boardId && (
          <div className="px-3 pb-2">
            {adding ? (
              <div className="flex gap-2">
                <div className="flex-1">
                  <Input
                    autoFocus
                    value={newTitle}
                    onChange={(e) => setNewTitle(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter' && newTitle.trim()) createMut.mutate(newTitle.trim())
                      if (e.key === 'Escape') { setAdding(false); setNewTitle('') }
                    }}
                    placeholder={t('card_title_ph')}
                  />
                </div>
                <Button size="sm" onClick={() => newTitle.trim() && createMut.mutate(newTitle.trim())}>{t('add')}</Button>
              </div>
            ) : (
              <button onClick={() => setAdding(true)} className="flex items-center gap-1.5 text-sm text-text-secondary hover:text-text-primary px-1.5 py-1">
                <Plus size={15} /> {t('new_task')}
              </button>
            )}
          </div>
        )}

        {tasks.length === 0 ? (
          <p className="text-center text-text-tertiary text-sm py-12">{t('no_tasks')}</p>
        ) : (
          <div className="bg-white rounded-xl border border-border overflow-hidden">
            {tasks.map(task => (
              <div key={task.id}>
                <TaskRow
                  task={task}
                  depth={0}
                  onOpen={selectTask}
                  onToggle={(tk) => toggleMut.mutate(tk)}
                  expandable={!!task.subtask_count}
                  expanded={expanded.has(task.id)}
                  onExpand={() => toggleExpand(task.id)}
                  onContextMenu={openTaskMenu}
                  accent={boards.find(b => b.id === task.board_id)?.color}
                />
                {expanded.has(task.id) && (
                  <Subtasks parentId={task.id} onOpen={selectTask} onToggle={(tk) => toggleMut.mutate(tk)} onContextMenu={openTaskMenu} accent={boards.find(b => b.id === task.board_id)?.color} />
                )}
              </div>
            ))}
          </div>
        )}
      </div>

      {taskMenu && <MenuDropdown items={taskMenuItems} pos={taskMenu.pos} onClose={() => setTaskMenu(null)} />}
      {confirmState && <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />}
    </div>
  )
}
