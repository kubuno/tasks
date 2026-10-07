import { create } from 'zustand'
import type { Collection } from './api'

export type TasksView = 'list' | 'kanban' | 'calendar'

export interface TasksFilters {
  assignee: string | null
  labelId: string | null
  status: string | null
}

const DEFAULT_FILTERS: TasksFilters = { assignee: null, labelId: null, status: null }

interface TasksState {
  currentBoardId: string | null
  view: TasksView
  collection: Collection
  searchQuery: string
  filters: TasksFilters
  selectedTaskId: string | null
  pendingCreateStackId: string | null

  setCurrentBoard: (id: string | null) => void
  setView: (v: TasksView) => void
  setCollection: (c: Collection) => void
  setSearchQuery: (q: string) => void
  setFilters: (f: Partial<TasksFilters>) => void
  clearFilters: () => void
  selectTask: (id: string | null) => void
  setPendingCreateStack: (id: string | null) => void
}

export const useTasksStore = create<TasksState>((set) => ({
  currentBoardId: null,
  view: 'kanban',
  collection: 'all',
  searchQuery: '',
  filters: { ...DEFAULT_FILTERS },
  selectedTaskId: null,
  pendingCreateStackId: null,

  setCurrentBoard: (currentBoardId) => set({ currentBoardId, selectedTaskId: null }),
  setView: (view) => set({ view }),
  setCollection: (collection) => set({ collection, currentBoardId: null, view: 'list' }),
  setSearchQuery: (searchQuery) => set({ searchQuery }),
  setFilters: (f) => set((s) => ({ filters: { ...s.filters, ...f } })),
  clearFilters: () => set({ filters: { ...DEFAULT_FILTERS } }),
  selectTask: (selectedTaskId) => set({ selectedTaskId }),
  setPendingCreateStack: (pendingCreateStackId) => set({ pendingCreateStackId }),
}))
