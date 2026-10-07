import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ChevronDown, ChevronUp, Star } from 'lucide-react'
import { EmptyState, Spinner } from '@ui'
import { toDate } from '@kubuno/sdk'
import { tasksApi, type Task } from './api'
import ListCard from './ListCard'
import TaskLine from './TaskLine'
import InlineComposer, { type DraftTask } from './InlineComposer'
import { AddTaskIcon, EmptyTasksArt } from './TaskIcons'
import { useListPrefs, type ListSortMode } from './listPrefs'
import { useTasksStore } from './store'

/** Where "recently starred" stops being recent. */
const RECENT_DAYS = 30

/** The scroll frame both views sit in: columns scroll sideways, cards inside. */
function Deck({ children }: { children: React.ReactNode }) {
  return (
    <div className="h-full overflow-x-auto overflow-y-hidden">
      <div className="flex h-full w-max min-w-full items-start justify-center gap-4 p-6">
        {children}
      </div>
    </div>
  )
}

/**
 * The overview: every list the reader has left visible, side by side.
 *
 * Each column owns its own query, so adding a task to one list does not
 * re-fetch the others, and a column can be hidden without any of the rest
 * noticing.
 */
export default function TasksOverview({ mode }: { mode: 'all' | 'starred' }) {
  const { t } = useTranslation('tasks')
  const qc = useQueryClient()
  const { hidden, sort, doneOpen, setHidden, setSort, setDoneOpen } = useListPrefs()
  const [dragCol, setDragCol] = useState<string | null>(null)
  const [overCol, setOverCol] = useState<string | null>(null)

  const { data: boards = [], isLoading } = useQuery({ queryKey: ['tasks-boards'], queryFn: tasksApi.listBoards })

  const visible = useMemo(
    () => boards.filter(b => !b.is_archived && !hidden.includes(b.id)),
    [boards, hidden],
  )

  const reorder = useMutation({
    mutationFn: (orderedIds: string[]) => tasksApi.reorderBoards(orderedIds),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['tasks-boards'] }),
  })

  /** Puts `moved` where `target` sits and renumbers the whole run. */
  const moveColumn = (moved: string, target: string) => {
    if (moved === target) return
    const ids = boards.filter(b => !b.is_archived).map(b => b.id)
    const from = ids.indexOf(moved)
    const to   = ids.indexOf(target)
    if (from < 0 || to < 0) return
    ids.splice(to, 0, ...ids.splice(from, 1))
    reorder.mutate(ids)
  }

  const moveFirst = (id: string) => {
    const ids = boards.filter(b => !b.is_archived).map(b => b.id)
    reorder.mutate([id, ...ids.filter(x => x !== id)])
  }

  if (mode === 'starred') return <StarredCard />

  if (isLoading) {
    return <div className="flex h-full items-center justify-center"><Spinner /></div>
  }

  if (visible.length === 0) {
    return (
      <div className="flex h-full items-center justify-center p-8">
        <EmptyState
          variant={boards.length === 0 ? 'first-use' : 'no-results'}
          icon={<EmptyTasksArt className="h-24 w-32" />}
          title={boards.length === 0 ? t('empty_title') : t('all_lists_hidden')}
          description={boards.length === 0 ? t('empty_desc') : t('all_lists_hidden_desc')}
          {...(boards.length > 0 ? {
            action: {
              label: t('show_all_lists'),
              variant: 'secondary' as const,
              onClick: () => { for (const id of hidden) void setHidden(id, false) },
            },
          } : {})}
        />
      </div>
    )
  }

  return (
    <Deck>
      {visible.map(board => (
        <ListCard
          key={board.id}
          board={board}
          sort={(sort[board.id] ?? 'custom') as ListSortMode}
          onSort={(m) => void setSort(board.id, m)}
          doneOpen={doneOpen[board.id] ?? false}
          onDoneOpen={(o) => void setDoneOpen(board.id, o)}
          onHide={() => void setHidden(board.id, true)}
          onMoveFirst={() => moveFirst(board.id)}
          dropTarget={overCol === board.id && dragCol !== null && dragCol !== board.id}
          onColumnDragStart={(e) => {
            // Same rule as the task rows: the payload carries the id, because a
            // drop firing in the dragstart tick would read stale React state.
            e.dataTransfer.setData('text/plain', `col:${board.id}`)
            e.dataTransfer.effectAllowed = 'move'
            setDragCol(board.id)
          }}
          onColumnDragOver={(e) => {
            if (!dragCol) return
            e.preventDefault()
            setOverCol(board.id)
          }}
          onColumnDrop={(e) => {
            const payload = e.dataTransfer.getData('text/plain')
            setDragCol(null); setOverCol(null)
            if (!payload.startsWith('col:')) return
            e.preventDefault()
            moveColumn(payload.slice(4), board.id)
          }}
        />
      ))}
    </Deck>
  )

  /**
   * Starred: a single wide card rather than columns, because what it gathers
   * cuts across the lists. Each row therefore says which list it came from.
   */
  function StarredCard() {
    const search = useTasksStore(s => s.searchQuery)
    const selectTask = useTasksStore(s => s.selectTask)
    const [composing, setComposing] = useState(false)
    const [oldOpen, setOldOpen] = useState(true)

    const { data: tasks = [], isLoading: loading } = useQuery({
      queryKey: ['tasks-starred', search],
      queryFn: () => tasksApi.listTasks({ collection: 'starred', search: search || undefined }),
    })

    const listName = useMemo(() => {
      const m = new Map<string, string>()
      for (const b of boards) m.set(b.id, b.is_default ? t('default_board') : b.title)
      return m
    }, [boards])

    const { recent, older } = useMemo(() => {
      const cut = Date.now() - RECENT_DAYS * 86_400_000
      const at = (x: Task) => (x.starred_at ? toDate(x.starred_at).getTime() : Date.now())
      const sorted = [...tasks].sort((a, b) => at(b) - at(a))
      return {
        recent: sorted.filter(x => at(x) >= cut),
        older:  sorted.filter(x => at(x) < cut),
      }
    }, [tasks])

    const refresh = () => {
      qc.invalidateQueries({ queryKey: ['tasks-starred'] })
      qc.invalidateQueries({ queryKey: ['tasks-card'] })
      qc.invalidateQueries({ queryKey: ['tasks-list'] })
    }
    const toggleDone = useMutation({
      mutationFn: (task: Task) => tasksApi.updateTask(task.id, { status: task.status === 'done' ? 'open' : 'done' }),
      onSuccess: refresh,
    })
    const unstar = useMutation({
      mutationFn: (task: Task) => tasksApi.updateTask(task.id, { starred: false }),
      onSuccess: refresh,
    })
    // A task typed here is starred from the outset, and lands in the inbox list.
    const create = useMutation({
      mutationFn: async (d: DraftTask) => {
        const target = boards.find(b => b.is_default) ?? boards.find(b => !b.is_archived)
        if (!target) return
        await tasksApi.createTask({
          board_id:    target.id,
          title:       d.title,
          description: d.description.trim() || null,
          due_at:      d.dueLocal ? new Date(d.dueLocal).toISOString() : null,
          rrule:       d.rrule,
          starred:     true,
        })
      },
      onSuccess: refresh,
    })

    const row = (task: Task) => (
      <TaskLine
        key={task.id}
        task={task}
        listName={listName.get(task.board_id)}
        onOpen={selectTask}
        onToggleDone={(x) => toggleDone.mutate(x)}
        onToggleStar={(x) => unstar.mutate(x)}
      />
    )

    return (
      <Deck>
        <section
          aria-label={t('view_starred')}
          className="flex max-h-full w-[560px] max-w-full flex-shrink-0 flex-col self-start
                     overflow-hidden rounded-xl border border-border bg-surface-0"
        >
          <header className="flex items-center gap-2 px-4 pb-1 pt-4">
            <Star size={18} className="text-primary" fill="currentColor" aria-hidden="true" />
            <h2 className="text-lg font-normal leading-7 text-text-nav">{t('view_starred')}</h2>
          </header>

          <button
            type="button"
            onClick={() => setComposing(true)}
            className={`mx-2 flex items-center gap-3 rounded-lg py-2 pl-2 pr-3 text-left text-sm
                        text-primary transition-colors
                        ${composing ? 'bg-primary-light/60' : 'hover:bg-primary-light/40'}`}
          >
            <AddTaskIcon />
            {t('add_starred_task')}
          </button>

          {composing && (
            <InlineComposer onSubmit={(d) => create.mutate(d)} onClose={() => setComposing(false)} />
          )}

          <div className="flex min-h-0 flex-1 flex-col overflow-y-auto px-2 pb-3 pt-1">
            {loading && <div className="flex justify-center py-6"><Spinner /></div>}

            {!loading && tasks.length === 0 && !composing && (
              <div className="flex flex-col items-center px-6 pb-8 pt-2 text-center">
                <EmptyTasksArt className="h-24 w-32" />
                <p className="mt-3 text-sm text-text-nav">{t('no_starred_title')}</p>
                <p className="mt-1 text-xs leading-5 text-text-secondary">{t('no_starred_desc')}</p>
              </div>
            )}

            {recent.length > 0 && (
              <>
                <div className="px-2 pb-1 pt-2 text-xs font-semibold uppercase tracking-wide text-text-secondary">
                  {t('starred_recently')}
                </div>
                {recent.map(row)}
              </>
            )}

            {older.length > 0 && (
              <>
                <button
                  type="button"
                  onClick={() => setOldOpen(v => !v)}
                  aria-expanded={oldOpen}
                  className="mt-1 flex items-center justify-between rounded-lg px-2 py-1.5 text-left
                             text-xs font-semibold uppercase tracking-wide text-text-secondary hover:bg-surface-1"
                >
                  {t('starred_long_ago')}
                  {oldOpen ? <ChevronUp className="h-3.5 w-3.5" /> : <ChevronDown className="h-3.5 w-3.5" />}
                </button>
                {oldOpen && older.map(row)}
              </>
            )}
          </div>
        </section>
      </Deck>
    )
  }
}
