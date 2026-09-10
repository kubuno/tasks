import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ChevronDown, ChevronRight, MoreVertical } from 'lucide-react'
import { MenuDropdown, Spinner, type MenuItem, type MenuDropdownPos } from '@ui'
import { prompt, useConfirm, navigate, toDate } from '@kubuno/sdk'
import { ConfirmDialog } from '@ui'
import { tasksApi, type Board, type Task } from './api'
import { AddTaskIcon, DragBar, EmptyTasksArt } from './TaskIcons'
import TaskLine from './TaskLine'
import InlineComposer, { type DraftTask } from './InlineComposer'
import { SORT_MODES, type ListSortMode } from './listPrefs'
import { useTasksStore } from './store'
import { printTaskList } from './printList'
import { isOverdue } from './helpers'

/** Tasks completed longer ago than this are what "clean up old tasks" removes. */
const STALE_DAYS = 30

interface Props {
  board: Board
  sort: ListSortMode
  onSort: (mode: ListSortMode) => void
  doneOpen: boolean
  onDoneOpen: (open: boolean) => void
  onHide: () => void
  onMoveFirst: () => void
  /** Reordering the COLUMN itself (the grab bar at the top of the card). */
  onColumnDragStart?: (e: React.DragEvent) => void
  onColumnDragOver?: (e: React.DragEvent) => void
  onColumnDrop?: (e: React.DragEvent) => void
  dropTarget?: boolean
}

/** Orders a list's tasks. `custom` keeps the server order (hand-made ranking). */
function sortTasks(tasks: Task[], mode: ListSortMode): Task[] {
  const out = [...tasks]
  switch (mode) {
    case 'created':
      return out.sort((a, b) => toDate(b.created_at).getTime() - toDate(a.created_at).getTime())
    case 'due':
      // Undated tasks sink to the bottom rather than pretending to be due now.
      return out.sort((a, b) => {
        if (!a.due_at && !b.due_at) return 0
        if (!a.due_at) return 1
        if (!b.due_at) return -1
        return toDate(a.due_at).getTime() - toDate(b.due_at).getTime()
      })
    case 'starred':
      return out.sort((a, b) => {
        if (a.starred !== b.starred) return a.starred ? -1 : 1
        const at = a.starred_at ? toDate(a.starred_at).getTime() : 0
        const bt = b.starred_at ? toDate(b.starred_at).getTime() : 0
        return bt - at
      })
    case 'title':
      return out.sort((a, b) => a.title.localeCompare(b.title))
    default:
      return out
  }
}

export default function ListCard({
  board, sort, onSort, doneOpen, onDoneOpen, onHide, onMoveFirst,
  onColumnDragStart, onColumnDragOver, onColumnDrop, dropTarget,
}: Props) {
  const { t } = useTranslation('tasks')
  const qc = useQueryClient()
  const selectTask = useTasksStore(s => s.selectTask)
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()
  const [menu, setMenu] = useState<MenuDropdownPos | null>(null)
  const [composing, setComposing] = useState(false)
  const [dragOverId, setDragOverId] = useState<string | null>(null)

  const key = ['tasks-card', board.id]
  const { data: all = [], isLoading } = useQuery({
    queryKey: key,
    // Subtasks come down with their parents so a column is one request.
    queryFn: () => tasksApi.listTasks({ board_id: board.id, include_subtasks: true }),
  })
  const refresh = () => {
    qc.invalidateQueries({ queryKey: ['tasks-card', board.id] })
    qc.invalidateQueries({ queryKey: ['tasks-list'] })
    qc.invalidateQueries({ queryKey: ['tasks-starred'] })
  }

  const { open, done, childrenOf } = useMemo(() => {
    const kids = new Map<string, Task[]>()
    for (const task of all) {
      if (!task.parent_task_id) continue
      const arr = kids.get(task.parent_task_id) ?? []
      arr.push(task)
      kids.set(task.parent_task_id, arr)
    }
    const roots = all.filter(x => !x.parent_task_id)
    return {
      open:        sortTasks(roots.filter(x => x.status !== 'done'), sort),
      done:        roots.filter(x => x.status === 'done'),
      childrenOf:  kids,
    }
  }, [all, sort])

  // A task whose date has passed is the one thing a list must not bury. It is
  // lifted out only under the hand-made ordering: an explicit sort (by title,
  // by date…) is an instruction, and silently overriding it would be wrong.
  const { late, rest } = useMemo(() => {
    if (sort !== 'custom') return { late: [] as Task[], rest: open }
    return {
      late: open.filter(x => isOverdue(x.due_at, x.status)),
      rest: open.filter(x => !isOverdue(x.due_at, x.status)),
    }
  }, [open, sort])

  const staleCount = useMemo(() => {
    const cut = Date.now() - STALE_DAYS * 86_400_000
    return all.filter(x => x.status === 'done' && x.completed_at && toDate(x.completed_at).getTime() < cut).length
  }, [all])

  // ── Mutations ──────────────────────────────────────────────────────────────
  const create = useMutation({
    mutationFn: (d: DraftTask) => tasksApi.createTask({
      board_id:    board.id,
      title:       d.title,
      description: d.description.trim() || null,
      due_at:      d.dueLocal ? new Date(d.dueLocal).toISOString() : null,
      rrule:       d.rrule,
    }),
    onSuccess: refresh,
  })
  const toggleDone = useMutation({
    mutationFn: (task: Task) =>
      tasksApi.updateTask(task.id, { status: task.status === 'done' ? 'open' : 'done' }),
    onSuccess: refresh,
  })
  const toggleStar = useMutation({
    mutationFn: (task: Task) => tasksApi.updateTask(task.id, { starred: !task.starred }),
    onSuccess: refresh,
  })
  const rename = useMutation({
    mutationFn: (title: string) => tasksApi.updateBoard(board.id, { title }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['tasks-boards'] }),
  })

  // ── Task drag and drop (hand-made order, and moving between lists) ──────────
  // The id travels in the drag payload, not in React state: a drop that follows
  // its dragstart in the same tick would read a state that has not landed yet.
  const onTaskDragStart = (e: React.DragEvent, task: Task) => {
    e.stopPropagation()
    e.dataTransfer.setData('text/plain', `task:${board.id}:${task.id}`)
    e.dataTransfer.effectAllowed = 'move'
  }
  const onTaskDragOver = (e: React.DragEvent, task: Task) => {
    if (!e.dataTransfer.types.includes('text/plain')) return
    e.preventDefault()
    e.stopPropagation()
    setDragOverId(task.id)
  }
  const onTaskDrop = async (e: React.DragEvent, target: Task) => {
    const payload = e.dataTransfer.getData('text/plain')
    setDragOverId(null)
    if (!payload.startsWith('task:')) return
    e.preventDefault()
    e.stopPropagation()
    const [, fromBoard, taskId] = payload.split(':')
    if (taskId === target.id) return

    if (fromBoard !== board.id) {
      await tasksApi.moveTasksToBoard([taskId], board.id)
    }
    // Slot the task just above the row it was dropped on.
    const idx = open.findIndex(x => x.id === target.id)
    const before = idx > 0 ? open[idx - 1].position : target.position - 1
    await tasksApi.moveTask(taskId, { position: (before + target.position) / 2 })
    if (fromBoard !== board.id) qc.invalidateQueries({ queryKey: ['tasks-card', fromBoard] })
    refresh()
  }

  // ── Bulk clean-ups offered by the column menu ──────────────────────────────
  const deleteMany = async (victims: Task[]) => {
    for (const v of victims) await tasksApi.deleteTask(v.id)
    refresh()
  }

  const doRename = async () => {
    const title = await prompt({
      title: t('rename_list'), placeholder: t('list_name'),
      defaultValue: board.title, confirmLabel: t('save'),
    })
    if (title?.trim()) rename.mutate(title.trim())
  }

  const doDelete = async () => {
    if (await confirm({
      title: t('delete_list'), message: t('confirm_delete_list'),
      confirmLabel: t('delete'), variant: 'danger',
    })) {
      await tasksApi.deleteBoard(board.id)
      qc.invalidateQueries({ queryKey: ['tasks-boards'] })
    }
  }

  const menuItems: MenuItem[] = [
    { type: 'label', text: t('sort_by') },
    ...SORT_MODES.map<MenuItem>(m => ({
      type: 'action', label: t(`sort_${m}`), checked: sort === m,
      onClick: () => { setMenu(null); onSort(m) },
    })),
    { type: 'separator' },
    // The default list is the inbox: the backend refuses to rename or remove it.
    { type: 'action', label: t('rename_list'), disabled: board.is_default, onClick: () => { setMenu(null); void doRename() } },
    { type: 'action', label: t('delete_list'), disabled: board.is_default, onClick: () => { setMenu(null); void doDelete() } },
    { type: 'action', label: t('move_list_first'), onClick: () => { setMenu(null); onMoveFirst() } },
    { type: 'action', label: t('hide_list'),       onClick: () => { setMenu(null); onHide() } },
    { type: 'separator' },
    { type: 'action', label: t('print_list'), onClick: () => { setMenu(null); printTaskList(board.is_default ? t('default_board') : board.title, open, done, childrenOf, t('completed_n', { count: done.length })) } },
    {
      type: 'action', label: t('delete_completed'), disabled: done.length === 0,
      onClick: async () => {
        setMenu(null)
        if (await confirm({ title: t('delete_completed'), message: t('confirm_delete_completed'), confirmLabel: t('delete'), variant: 'danger' })) {
          await deleteMany(all.filter(x => x.status === 'done'))
        }
      },
    },
    {
      type: 'action', label: t('cleanup_old'), disabled: staleCount === 0,
      onClick: async () => {
        setMenu(null)
        if (await confirm({ title: t('cleanup_old'), message: t('confirm_cleanup_old', { count: staleCount, days: STALE_DAYS }), confirmLabel: t('delete'), variant: 'danger' })) {
          const cut = Date.now() - STALE_DAYS * 86_400_000
          await deleteMany(all.filter(x => x.status === 'done' && x.completed_at && toDate(x.completed_at).getTime() < cut))
        }
      },
    },
    // A board laid out in columns keeps its own page; the overview cannot show stacks.
    ...(board.board_type === 'kanban'
      ? [{ type: 'separator' as const },
         { type: 'action' as const, label: t('open_board'), onClick: () => { setMenu(null); navigate(`/tasks/boards/${board.id}`) } }]
      : []),
  ]

  const renderTask = (task: Task, depth: number): React.ReactNode => (
    <div key={task.id}>
      <TaskLine
        task={task}
        depth={depth}
        onOpen={selectTask}
        onToggleDone={(x) => toggleDone.mutate(x)}
        onToggleStar={(x) => toggleStar.mutate(x)}
        draggable={sort === 'custom'}
        onDragStart={onTaskDragStart}
        onDragOver={onTaskDragOver}
        onDrop={onTaskDrop}
        dropBefore={dragOverId === task.id}
      />
      {(childrenOf.get(task.id) ?? []).map(child => renderTask(child, depth + 1))}
    </div>
  )

  const isEmpty = open.length === 0 && done.length === 0
  // Everything ticked off is a different message from an untouched list.
  const allDone = open.length === 0 && done.length > 0

  return (
    <section
      onDragOver={onColumnDragOver}
      onDrop={onColumnDrop}
      onDragLeave={() => setDragOverId(null)}
      aria-label={board.is_default ? t('default_board') : board.title}
      className={`group/card flex max-h-full w-[400px] flex-shrink-0 flex-col self-start overflow-hidden
                  rounded-xl border bg-surface-0 ${dropTarget ? 'border-primary' : 'border-border'}`}
    >
      {/* Grab bar — reorders the column. Only shown once the pointer is over the card. */}
      <div
        draggable
        onDragStart={onColumnDragStart}
        title={t('reorder_list')}
        className="flex h-3 cursor-grab items-center justify-center pt-1.5 opacity-0
                   transition-opacity group-hover/card:opacity-100 active:cursor-grabbing"
      >
        <DragBar />
      </div>

      <header className="flex items-start gap-2 px-4 pb-1 pt-2">
        <h2 className="min-w-0 flex-1 truncate text-lg font-normal leading-7 text-text-nav">
          {board.is_default ? t('default_board') : board.title}
        </h2>
        <button
          type="button"
          onClick={(e) => { const r = e.currentTarget.getBoundingClientRect(); setMenu({ top: r.bottom + 4, left: Math.max(8, r.right - 264), minWidth: 264 }) }}
          title={t('list_options')}
          aria-label={t('list_options')}
          className="mt-0.5 rounded-full p-1.5 text-text-secondary hover:bg-surface-2 hover:text-text-primary"
        >
          <MoreVertical size={18} />
        </button>
      </header>

      <button
        type="button"
        onClick={() => setComposing(true)}
        className={`mx-2 flex items-center gap-3 rounded-lg py-2 pl-2 pr-3 text-left text-sm
                    text-primary transition-colors ${composing ? 'bg-primary-light/60' : 'hover:bg-primary-light/40'}`}
      >
        <AddTaskIcon />
        {t('add_task')}
      </button>

      {composing && (
        <InlineComposer
          onSubmit={(d) => create.mutate(d)}
          onClose={() => setComposing(false)}
        />
      )}

      <div className="flex min-h-0 flex-1 flex-col overflow-y-auto px-2 pb-2 pt-1">
        {isLoading && <div className="flex justify-center py-6"><Spinner /></div>}

        {!isLoading && !composing && (isEmpty || allDone) && (
          <div className="flex flex-col items-center px-6 pb-6 pt-4 text-center">
            <EmptyTasksArt className="h-24 w-32" />
            <p className="mt-3 text-sm text-text-nav">{allDone ? t('all_done_title') : t('empty_title')}</p>
            <p className="mt-1 text-xs leading-5 text-text-secondary">{allDone ? t('all_done_desc') : t('empty_desc')}</p>
          </div>
        )}

        {late.length > 0 && (
          <>
            <div className="px-2 pb-1 pt-2 text-xs font-semibold text-danger">{t('past_due')}</div>
            {late.map(task => renderTask(task, 0))}
          </>
        )}

        {rest.map(task => renderTask(task, 0))}

        {done.length > 0 && (
          <>
            <button
              type="button"
              onClick={() => onDoneOpen(!doneOpen)}
              aria-expanded={doneOpen}
              className="mt-1 flex items-center gap-1 rounded-lg px-2 py-1.5 text-left text-xs
                         font-medium text-text-secondary hover:bg-surface-1"
            >
              {doneOpen ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
              {t('completed_n', { count: done.length })}
            </button>
            {doneOpen && done.map(task => renderTask(task, 0))}
          </>
        )}
      </div>

      {menu && <MenuDropdown items={menuItems} pos={menu} onClose={() => setMenu(null)} />}
      {confirmState && <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />}
    </section>
  )
}
