import { SidebarNavItem, prompt, navigate } from '@kubuno/sdk'
import { useEffect, useMemo, useState } from 'react'
import { useLocation, useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import {
  CalendarClock, CalendarDays, AlertTriangle, Star, CheckCircle2, ListTodo,
  ChevronDown, ChevronUp, Plus, Columns3,
} from 'lucide-react'
import { Checkbox } from '@ui'
import { tasksApi, type Collection } from './api'
import { useTasksStore } from './store'
import { useListPrefs } from './listPrefs'
import { hashTo, fromHash } from './hashRoute'

// Every clickable element of this sidebar is an <a> carrying a real href.
// Views have no route of their own, so they are addressable through the URL
// hash (`/tasks/#collection/<key>`, cf. hashRoute.ts) and the selection is read
// back from `useLocation().hash` — direct links and Back therefore work.
const VIEWS: { key: Collection; icon: React.ReactNode }[] = [
  { key: 'all',     icon: <CheckCircle2 size={18} /> },
  { key: 'starred', icon: <Star size={18} /> },
]

/** Date-driven views. Secondary to the lists, so they fold away by default. */
const FILTERS: { key: Collection; icon: React.ReactNode }[] = [
  { key: 'today',     icon: <CalendarDays size={18} /> },
  { key: 'upcoming',  icon: <CalendarClock size={18} /> },
  { key: 'overdue',   icon: <AlertTriangle size={18} /> },
  { key: 'important', icon: <ListTodo size={18} /> },
  { key: 'completed', icon: <CheckCircle2 size={18} /> },
]

const ALL_KEYS = [...VIEWS, ...FILTERS].map(v => v.key)

export default function TasksSidebarBody({ collapsed = false }: { collapsed?: boolean }) {
  const { t } = useTranslation('tasks')
  const qc = useQueryClient()
  const params = useParams()
  const { hash } = useLocation()
  const activeBoardId = params.id ?? null
  const collection = useTasksStore(s => s.collection)
  const setCollection = useTasksStore(s => s.setCollection)
  const { hidden, listsOpen, isHidden, setHidden, setListsOpen } = useListPrefs()
  const [filtersOpen, setFiltersOpen] = useState(false)

  const { data: boards = [] } = useQuery({ queryKey: ['tasks-boards'], queryFn: tasksApi.listBoards })

  // One request feeds every counter: the badge next to a list is the number of
  // tasks left to do in it. Keyed under `tasks-list` so the views that create or
  // complete a task already invalidate it.
  const { data: allTasks = [] } = useQuery({
    queryKey: ['tasks-list', 'sidebar-counts'],
    queryFn: () => tasksApi.listTasks({}),
  })
  const counts = useMemo(() => {
    const m = new Map<string, number>()
    for (const task of allTasks) {
      if (task.status === 'done' || task.status === 'cancelled') continue
      m.set(task.board_id, (m.get(task.board_id) ?? 0) + 1)
    }
    return m
  }, [allTasks])

  // The hash drives the view: opening `/tasks/#collection/starred` directly, or
  // pressing Back after a change, applies it to the store the views read.
  useEffect(() => {
    const parsed = fromHash(hash)
    if (parsed?.kind !== 'collection') return
    if (ALL_KEYS.includes(parsed.id as Collection)) setCollection(parsed.id as Collection)
  }, [hash, setCollection])

  // A filter chosen from a deep link must not stay hidden behind a folded section.
  useEffect(() => {
    if (FILTERS.some(f => f.key === collection)) setFiltersOpen(true)
  }, [collection])

  const createList = async () => {
    const title = await prompt({ title: t('new_list'), placeholder: t('list_name'), confirmLabel: t('create_action') })
    if (!title?.trim()) return
    const board = await tasksApi.createBoard({ title: title.trim(), board_type: 'list' })
    qc.invalidateQueries({ queryKey: ['tasks-boards'] })
    // A brand new list is visible: make sure a stale hidden entry cannot bury it.
    if (isHidden(board.id)) void setHidden(board.id, false)
    navigate(hashTo('collection', 'all'))
  }

  const sectionHeader = (label: string, open: boolean, onToggle: () => void) => (
    <button
      type="button"
      onClick={onToggle}
      aria-expanded={open}
      className="mt-3 flex w-full items-center justify-between rounded-lg px-3 py-1.5 text-left
                 text-sm font-semibold text-text-secondary hover:bg-surface-1"
    >
      {label}
      {open ? <ChevronUp className="h-3.5 w-3.5" /> : <ChevronDown className="h-3.5 w-3.5" />}
    </button>
  )

  return (
    <div className="flex flex-col gap-0.5 px-2 py-2">
      {VIEWS.map(v => (
        <SidebarNavItem
          key={v.key}
          label={t(`view_${v.key}`)}
          icon={v.icon}
          collapsed={collapsed}
          active={!activeBoardId && collection === v.key}
          to={hashTo('collection', v.key)}
        />
      ))}

      {!collapsed && (
        <>
          {sectionHeader(t('lists'), listsOpen, () => void setListsOpen(!listsOpen))}

          {listsOpen && (
            <div className="flex flex-col">
              {boards.filter(b => !b.is_archived).map(b => {
                const n = counts.get(b.id) ?? 0
                const shown = !hidden.includes(b.id)
                return (
                  <div
                    key={b.id}
                    className="group flex items-center gap-2 rounded-lg py-1.5 pl-3 pr-2 hover:bg-surface-1"
                  >
                    {/* The box says whether this list has a column in the overview. */}
                    <Checkbox
                      checked={shown}
                      onChange={(v) => void setHidden(b.id, !v)}
                      color={b.color}
                      label={b.is_default ? t('default_board') : b.title}
                      className="min-w-0 flex-1"
                      labelClassName="truncate text-sm text-text-nav"
                    />
                    {n > 0 && <span className="flex-shrink-0 text-xs text-text-tertiary">{n}</span>}
                    {b.board_type === 'kanban' && (
                      <a
                        href={`/tasks/boards/${b.id}`}
                        title={t('open_board')}
                        aria-label={t('open_board')}
                        onClick={(e) => { e.preventDefault(); navigate(`/tasks/boards/${b.id}`) }}
                        className="flex-shrink-0 rounded p-0.5 text-text-tertiary opacity-0
                                   hover:text-text-primary group-hover:opacity-100 focus-visible:opacity-100"
                      >
                        <Columns3 size={15} />
                      </a>
                    )}
                  </div>
                )
              })}

              <button
                type="button"
                onClick={() => void createList()}
                className="mt-0.5 flex items-center gap-3 rounded-lg py-2 pl-3 pr-2 text-left text-sm
                           text-text-nav hover:bg-surface-1"
              >
                <Plus size={18} className="text-text-secondary" />
                {t('create_list')}
              </button>
            </div>
          )}

          {sectionHeader(t('filters'), filtersOpen, () => setFiltersOpen(!filtersOpen))}
          {filtersOpen && FILTERS.map(f => (
            <SidebarNavItem
              key={f.key}
              label={t(`collection_${f.key}`)}
              icon={f.icon}
              collapsed={collapsed}
              active={!activeBoardId && collection === f.key}
              to={hashTo('collection', f.key)}
            />
          ))}
        </>
      )}
    </div>
  )
}
