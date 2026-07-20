import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { Plus, MoreVertical, Trash2, Pencil, ListPlus, X } from 'lucide-react'
import { MenuDropdown, type MenuItem, Input, Textarea, Button } from '@ui'
import { ConfirmDialog } from '@ui'
import { useConfirm } from '@kubuno/sdk'
import { prompt } from '@kubuno/sdk'
import { tasksApi, type Task, type Stack } from './api'
import { useTasksStore } from './store'
import TaskCard from './TaskCard'
import { buildTaskMenu } from './taskMenu'
import { copyKubunoData, openLabelPicker } from './kubunoData'
import { taskEnvelope } from './TasksDataCard'

interface Props {
  boardId: string
}

export default function TasksKanbanBoard({ boardId }: Props) {
  const { t } = useTranslation('tasks')
  const qc = useQueryClient()
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()
  const selectTask = useTasksStore(s => s.selectTask)
  const [dragId, setDragId] = useState<string | null>(null)
  const [dragOverStack, setDragOverStack] = useState<string | null>(null)
  const [addingIn, setAddingIn] = useState<string | null>(null)
  const [newCardTitle, setNewCardTitle] = useState('')
  const [addingColumn, setAddingColumn] = useState(false)
  const [newColTitle, setNewColTitle] = useState('')
  const [stackMenu, setStackMenu] = useState<{ stack: Stack; pos: { top: number; left: number } } | null>(null)
  const [taskMenu, setTaskMenu] = useState<{ task: Task; pos: { top: number; left: number } } | null>(null)
  const [selected, setSelected] = useState<Set<string>>(new Set())
  const [moveBar, setMoveBar] = useState<{ top: number; left: number } | null>(null)

  const { data: stacks = [], isLoading: loadingStacks } = useQuery({
    queryKey: ['tasks-stacks', boardId], queryFn: () => tasksApi.listStacks(boardId),
  })
  const { data: tasks = [], isLoading: loadingTasks } = useQuery({
    queryKey: ['tasks-board', boardId], queryFn: () => tasksApi.listTasks({ board_id: boardId }),
  })
  const { data: boards = [] } = useQuery({ queryKey: ['tasks-boards'], queryFn: tasksApi.listBoards })

  const invalidate = () => {
    qc.invalidateQueries({ queryKey: ['tasks-board', boardId] })
    qc.invalidateQueries({ queryKey: ['tasks-stacks', boardId] })
    qc.invalidateQueries({ queryKey: ['tasks-list'] })
  }
  const invalidateAllBoards = () => {
    invalidate()
    qc.invalidateQueries({ queryKey: ['tasks-board'] })
  }

  const moveMut = useMutation({
    mutationFn: (v: { id: string; stackId: string | null; position: number }) =>
      tasksApi.moveTask(v.id, { stack_id: v.stackId, position: v.position }),
    onSuccess: invalidate,
  })
  const completeMut = useMutation({
    mutationFn: (task: Task) =>
      task.status === 'done' ? tasksApi.updateTask(task.id, { status: 'open' }) : tasksApi.completeTask(task.id),
    onSuccess: invalidate,
  })
  const createMut = useMutation({
    mutationFn: (v: { stackId: string; title: string }) =>
      tasksApi.createTask({ board_id: boardId, stack_id: v.stackId, title: v.title }),
    onSuccess: () => { setNewCardTitle(''); setAddingIn(null); invalidate() },
  })
  const createColMut = useMutation({
    mutationFn: (title: string) => tasksApi.createStack(boardId, { title }),
    onSuccess: () => { setNewColTitle(''); setAddingColumn(false); invalidate() },
  })
  const delColMut = useMutation({ mutationFn: (id: string) => tasksApi.deleteStack(id), onSuccess: invalidate })
  const renameColMut = useMutation({
    mutationFn: (v: { id: string; title: string }) => tasksApi.updateStack(v.id, { title: v.title }),
    onSuccess: invalidate,
  })
  const setPriorityMut = useMutation({
    mutationFn: (v: { id: string; priority: number }) => tasksApi.updateTask(v.id, { priority: v.priority }),
    onSuccess: invalidate,
  })
  const deleteTaskMut = useMutation({ mutationFn: (id: string) => tasksApi.deleteTask(id), onSuccess: invalidate })
  const moveToBoardMut = useMutation({
    mutationFn: (v: { ids: string[]; boardId: string }) => tasksApi.moveTasksToBoard(v.ids, v.boardId),
    onSuccess: () => { setSelected(new Set()); invalidateAllBoards() },
  })

  if (loadingStacks || loadingTasks) {
    return <div className="flex items-center justify-center h-full"><div className="animate-pulse text-text-tertiary text-sm">…</div></div>
  }

  const tasksByStack = (stackId: string) =>
    tasks.filter(t => t.stack_id === stackId).sort((a, b) => a.position - b.position)
  const unsorted = tasks.filter(t => t.stack_id === null)
  const selectionActive = selected.size > 0
  const boardColor = boards.find(b => b.id === boardId)?.color

  const toggleSelect = (task: Task) => setSelected(prev => {
    const n = new Set(prev); n.has(task.id) ? n.delete(task.id) : n.add(task.id); return n
  })

  const onDrop = (e: React.DragEvent, stackId: string | null) => {
    const id = e.dataTransfer.getData('text/plain') || dragId
    if (!id) return
    const colTasks = stackId ? tasksByStack(stackId) : unsorted
    const last = colTasks[colTasks.length - 1]
    const position = last ? last.position + 1 : 0
    moveMut.mutate({ id, stackId, position })
    setDragId(null); setDragOverStack(null)
  }

  // ── Actions du menu contextuel d'une tâche ──
  const taskActions = {
    onOpen: selectTask,
    onToggleDone: (task: Task) => completeMut.mutate(task),
    onSetPriority: (id: string, priority: number) => setPriorityMut.mutate({ id, priority }),
    onMoveToBoard: (id: string, bId: string) => moveToBoardMut.mutate({ ids: [id], boardId: bId }),
    onAddSubtask: async (task: Task) => {
      const title = await prompt({ title: t('add_subtask'), placeholder: t('title'), confirmLabel: t('add') })
      if (title?.trim()) { await tasksApi.createSubtask(task.id, { board_id: boardId, title: title.trim() }); invalidate() }
    },
    onExportIcs: (task: Task) => window.open(`/api/v1/tasks/tasks/${task.id}/ics`, '_blank'),
    onCopyCard: (task: Task) => { copyKubunoData(taskEnvelope(task)).catch(() => {}) },
    onKubunoLabels: (task: Task) => { openLabelPicker(taskEnvelope(task)).catch(() => {}) },
    onDelete: async (task: Task) => {
      if (await confirm({ title: t('delete_task'), message: t('confirm_delete_task'), confirmLabel: t('delete'), variant: 'danger' }))
        deleteTaskMut.mutate(task.id)
    },
  }

  const column = (stack: Stack | null, title: string, list: Task[]) => {
    const key = stack ? stack.id : '__unsorted'
    return (
      <div
        key={key}
        onDragOver={(e) => { e.preventDefault(); setDragOverStack(key) }}
        onDragLeave={() => setDragOverStack(prev => (prev === key ? null : prev))}
        onDrop={(e) => onDrop(e, stack ? stack.id : null)}
        className={`flex flex-col w-72 flex-shrink-0 bg-surface-1 rounded-xl max-h-full ${dragOverStack === key ? 'ring-2 ring-primary' : ''}`}
      >
        <div className="flex items-center justify-between px-3 py-2.5">
          <div className="flex items-center gap-2 min-w-0">
            <span className="text-sm font-semibold text-text-primary truncate">{title}</span>
            <span className="text-xs text-text-tertiary">{list.length}</span>
          </div>
          {stack && (
            <button
              onClick={(e) => setStackMenu({ stack, pos: { top: e.clientY, left: e.clientX } })}
              className="text-text-tertiary hover:text-text-primary p-1 rounded hover:bg-surface-2"
            ><MoreVertical size={15} /></button>
          )}
        </div>

        <div className="flex-1 overflow-y-auto px-2 pb-2 space-y-2 min-h-[40px]">
          {list.map(task => (
            <TaskCard
              key={task.id}
              task={task}
              accent={task.color ?? boardColor}
              draggable={!selectionActive}
              onDragStart={(e, tk) => { setDragId(tk.id); e.dataTransfer.setData('text/plain', tk.id); e.dataTransfer.effectAllowed = 'move' }}
              onOpen={selectTask}
              onToggleComplete={(tk) => completeMut.mutate(tk)}
              onContextMenu={(e, tk) => { e.preventDefault(); setTaskMenu({ task: tk, pos: { top: e.clientY, left: e.clientX } }) }}
              selectionActive={selectionActive}
              selected={selected.has(task.id)}
              onToggleSelect={toggleSelect}
            />
          ))}
        </div>

        {stack && (
          <div className="px-2 pb-2">
            {addingIn === stack.id ? (
              <div className="space-y-1.5">
                <Textarea
                  autoFocus value={newCardTitle} onChange={(e) => setNewCardTitle(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); if (newCardTitle.trim()) createMut.mutate({ stackId: stack.id, title: newCardTitle.trim() }) }
                    if (e.key === 'Escape') { setAddingIn(null); setNewCardTitle('') }
                  }}
                  placeholder={t('card_title_ph')}
                  className="h-auto min-h-0 resize-none" rows={2}
                />
                <div className="flex gap-2">
                  <Button size="sm" onClick={() => newCardTitle.trim() && createMut.mutate({ stackId: stack.id, title: newCardTitle.trim() })}>{t('add')}</Button>
                  <Button variant="ghost" size="sm" onClick={() => { setAddingIn(null); setNewCardTitle('') }}>{t('cancel')}</Button>
                </div>
              </div>
            ) : (
              <button onClick={() => { setAddingIn(stack.id); setNewCardTitle('') }} className="flex items-center gap-1.5 text-sm text-text-secondary hover:text-text-primary w-full px-1.5 py-1 rounded hover:bg-surface-2">
                <Plus size={15} /> {t('add_card')}
              </button>
            )}
          </div>
        )}
      </div>
    )
  }

  const stackMenuItems: MenuItem[] = stackMenu ? [
    { type: 'action', label: t('add_card'), icon: <ListPlus size={15} />, onClick: () => { const s = stackMenu.stack; setStackMenu(null); setAddingIn(s.id); setNewCardTitle('') } },
    { type: 'action', label: t('rename'), icon: <Pencil size={15} />, onClick: async () => {
      const s = stackMenu.stack; setStackMenu(null)
      const title = await prompt({ title: t('rename'), defaultValue: s.title, confirmLabel: t('rename') })
      if (title?.trim() && title.trim() !== s.title) renameColMut.mutate({ id: s.id, title: title.trim() })
    } },
    { type: 'separator' },
    { type: 'action', label: t('delete_column'), icon: <Trash2 size={15} />, onClick: async () => {
      const s = stackMenu.stack; setStackMenu(null)
      if (await confirm({ title: t('delete_column'), message: t('confirm_delete_column'), confirmLabel: t('delete'), variant: 'danger' })) delColMut.mutate(s.id)
    } },
  ] : []

  const taskMenuItems: MenuItem[] = taskMenu ? buildTaskMenu(taskMenu.task, boards, t, taskActions) : []

  const moveBarItems: MenuItem[] = boards.filter(b => b.id !== boardId && !b.is_archived).map(b => ({
    type: 'action' as const,
    label: b.is_default ? t('default_board') : b.title,
    onClick: () => { moveToBoardMut.mutate({ ids: [...selected], boardId: b.id }); setMoveBar(null) },
  }))

  return (
    <div className="h-full flex flex-col">
      <div className="flex-1 overflow-x-auto p-4">
        <div className="flex gap-3 h-full items-start">
          {unsorted.length > 0 && column(null, t('unsorted'), unsorted)}
          {stacks.map(s => column(s, s.title, tasksByStack(s.id)))}

          <div className="w-72 flex-shrink-0">
            {addingColumn ? (
              <div className="bg-surface-1 rounded-xl p-2 space-y-1.5">
                <Input
                  autoFocus value={newColTitle} onChange={(e) => setNewColTitle(e.target.value)}
                  onKeyDown={(e) => { if (e.key === 'Enter' && newColTitle.trim()) createColMut.mutate(newColTitle.trim()); if (e.key === 'Escape') { setAddingColumn(false); setNewColTitle('') } }}
                  placeholder={t('column_title_ph')}
                />
                <div className="flex gap-2">
                  <Button size="sm" onClick={() => newColTitle.trim() && createColMut.mutate(newColTitle.trim())}>{t('add')}</Button>
                  <Button variant="ghost" size="sm" onClick={() => { setAddingColumn(false); setNewColTitle('') }}>{t('cancel')}</Button>
                </div>
              </div>
            ) : (
              <button onClick={() => setAddingColumn(true)} className="flex items-center gap-1.5 text-sm text-text-secondary hover:text-text-primary w-full px-3 py-2.5 rounded-xl bg-surface-1 hover:bg-surface-2">
                <Plus size={16} /> {t('add_column')}
              </button>
            )}
          </div>
        </div>
      </div>

      {/* Barre de sélection multiple */}
      {selectionActive && (
        <div className="flex items-center gap-3 px-4 py-2.5 border-t border-border bg-white shadow-[0_-2px_8px_rgba(0,0,0,0.04)]">
          <span className="text-sm font-medium text-text-primary">{t('selected_n', { count: selected.size })}</span>
          <button
            onClick={(e) => setMoveBar({ top: e.clientY - 8, left: e.clientX })}
            className="text-sm px-3 py-1.5 rounded-lg bg-primary text-white hover:bg-primary-hover"
          >{t('move_to_board')}</button>
          <button onClick={() => setSelected(new Set())} className="text-sm px-2 py-1.5 text-text-secondary hover:text-text-primary flex items-center gap-1">
            <X size={15} /> {t('clear_selection')}
          </button>
        </div>
      )}

      {stackMenu && <MenuDropdown items={stackMenuItems} pos={stackMenu.pos} onClose={() => setStackMenu(null)} />}
      {taskMenu && <MenuDropdown items={taskMenuItems} pos={taskMenu.pos} onClose={() => setTaskMenu(null)} />}
      {moveBar && moveBarItems.length > 0 && <MenuDropdown items={moveBarItems} pos={moveBar} onClose={() => setMoveBar(null)} />}
      {confirmState && <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />}
    </div>
  )
}
