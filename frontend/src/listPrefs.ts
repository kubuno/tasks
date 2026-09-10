import { useCallback, useEffect, useRef } from 'react'
import { useModulePrefs } from './userPrefs'

/**
 * How one list orders its tasks.
 *
 * `custom` is the hand-made order (drag and drop); the others are computed from
 * a task field. The choice belongs to the READER, not to the list: two people
 * sharing a list can each sort it their own way, so it lives in the per-user
 * preference bag rather than on the board row.
 */
export type ListSortMode = 'custom' | 'created' | 'due' | 'starred' | 'title'

export const SORT_MODES: ListSortMode[] = ['custom', 'created', 'due', 'starred', 'title']

export interface TasksListPrefs extends Record<string, unknown> {
  /** Ids of the lists whose column is hidden from the overview. */
  hidden: string[]
  /** List id → chosen ordering (absent = `custom`). */
  sort: Record<string, ListSortMode>
  /** Is the sidebar's "Lists" section unfolded? */
  listsOpen: boolean
  /** Are the completed tasks of a list unfolded? (list id → open) */
  doneOpen: Record<string, boolean>
}

const DEFAULTS: TasksListPrefs = { hidden: [], sort: {}, listsOpen: true, doneOpen: {} }

/**
 * Reads and writes the per-user list preferences.
 *
 * Mutations are read-modify-write on values the render closure captured, so a
 * burst of clicks (unchecking three lists in a row) would each start from the
 * same stale base and the last one would win. A synchronous ref, resynchronised
 * whenever the stored value changes, gives every mutation the previous one's
 * result — the write chain in `useModulePrefs` then persists them in order.
 */
export function useListPrefs() {
  const { prefs, update } = useModulePrefs<TasksListPrefs>('tasks_lists', DEFAULTS)

  const ref = useRef(prefs)
  useEffect(() => { ref.current = prefs }, [prefs])

  const isHidden = useCallback((id: string) => (ref.current.hidden ?? []).includes(id), [])

  const setHidden = useCallback((id: string, hidden: boolean) => {
    const cur = ref.current.hidden ?? []
    const next = hidden ? [...new Set([...cur, id])] : cur.filter(x => x !== id)
    ref.current = { ...ref.current, hidden: next }
    return update({ hidden: next })
  }, [update])

  const setSort = useCallback((id: string, mode: ListSortMode) => {
    const next = { ...(ref.current.sort ?? {}), [id]: mode }
    ref.current = { ...ref.current, sort: next }
    return update({ sort: next })
  }, [update])

  const setListsOpen = useCallback((open: boolean) => {
    ref.current = { ...ref.current, listsOpen: open }
    return update({ listsOpen: open })
  }, [update])

  const setDoneOpen = useCallback((id: string, open: boolean) => {
    const next = { ...(ref.current.doneOpen ?? {}), [id]: open }
    ref.current = { ...ref.current, doneOpen: next }
    return update({ doneOpen: next })
  }, [update])

  return {
    hidden:    prefs.hidden ?? [],
    sort:      prefs.sort ?? {},
    listsOpen: prefs.listsOpen !== false,
    doneOpen:  prefs.doneOpen ?? {},
    isHidden, setHidden, setSort, setListsOpen, setDoneOpen,
  }
}
