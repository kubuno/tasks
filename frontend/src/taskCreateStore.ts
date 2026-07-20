/**
 * Store backing the globally-mounted `TaskCreateDialog` (slot `app-dialogs`).
 *
 * Same pattern as contacts' `contactPickerStore`: the promise returned by
 * `createTask()` is parked in the store and settled by `_resolve()` when the
 * dialog closes. This lets ANY module (chat…) create a task through
 * `ModuleServiceRegistry.call('tasks', 'createTask')` without ever importing
 * tasks' code — when tasks is not installed, the service is simply absent.
 */
import { create } from 'zustand'

export interface CreateTaskOptions {
  /** Pre-fills the task title (consumers may pass e.g. a chat message body). */
  title?: string
}

/** What a consumer gets back: the created task, or null if the user cancelled. */
export interface CreatedTask {
  id: string
  title: string
}

interface TaskCreateState {
  createOpts: CreateTaskOptions | null
  /** Settles the pending `createTask()` promise and closes the dialog. */
  _resolve: (task: CreatedTask | null) => void
  /** Opens the dialog; resolves with the created task, or null on cancel. */
  createTask: (opts?: CreateTaskOptions) => Promise<CreatedTask | null>
}

export const useTaskCreateStore = create<TaskCreateState>((set, get) => {
  let pending: ((task: CreatedTask | null) => void) | null = null

  return {
    createOpts: null,

    _resolve: (task) => {
      const resolve = pending
      pending = null
      set({ createOpts: null })
      resolve?.(task)
    },

    createTask: (opts = {}) => {
      // A second call supersedes a pending one: cancel it rather than leak it.
      if (pending) get()._resolve(null)
      return new Promise<CreatedTask | null>((resolve) => {
        pending = resolve
        set({ createOpts: opts })
      })
    },
  }
})

/** Convenience wrapper published on the `tasks` service registry. */
export function createTask(opts?: CreateTaskOptions): Promise<CreatedTask | null> {
  return useTaskCreateStore.getState().createTask(opts)
}
