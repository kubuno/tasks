import { api as apiClient } from '@kubuno/sdk'

// ── Types ─────────────────────────────────────────────────────────────────────

export interface Board {
  id: string
  owner_id: string
  title: string
  description: string | null
  color: string
  board_type: 'kanban' | 'list'
  is_default: boolean
  is_archived: boolean
  sort_order: number
  caldav_token: string
  ctag: string
  created_at: string
  updated_at: string
}

export interface BoardShare {
  id: string
  board_id: string
  shared_with: string
  permission: 'read' | 'write' | 'admin'
  created_at: string
}

export interface Stack {
  id: string
  board_id: string
  title: string
  sort_order: number
  created_at: string
  updated_at: string
}

export interface Label {
  id: string
  board_id: string
  title: string
  color: string
  created_at: string
}

export type TaskStatus = 'open' | 'in_progress' | 'done' | 'cancelled'

export interface Task {
  id: string
  board_id: string
  stack_id: string | null
  parent_task_id: string | null
  owner_id: string
  title: string
  description: string | null
  status: TaskStatus
  priority: number
  percent_complete: number
  due_at: string | null
  start_at: string | null
  completed_at: string | null
  all_day: boolean
  color: string | null
  rrule: string | null
  reminders: unknown[]
  ical_uid: string
  etag: string
  sequence: number
  sort_order: number
  position: number
  linked_event_id: string | null
  linked_file_ids: string[]
  created_at: string
  updated_at: string
  // enrichis via get_with_meta (TaskWithMeta côté backend, champs aplatis)
  labels?: Label[]
  assignees?: string[]
  subtask_count?: number
  comment_count?: number
}

export interface Comment {
  id: string
  task_id: string
  author_id: string
  body: string
  created_at: string
  updated_at: string
}

export interface Attachment {
  id: string
  task_id: string
  file_id: string | null
  filename: string
  mime_type: string | null
  size_bytes: number | null
  created_at: string
}

export interface CreateTaskInput {
  board_id: string
  stack_id?: string | null
  parent_task_id?: string | null
  title: string
  description?: string | null
  status?: TaskStatus
  priority?: number
  percent_complete?: number
  due_at?: string | null
  start_at?: string | null
  all_day?: boolean
  color?: string | null
  rrule?: string | null
  reminders?: unknown[]
  label_ids?: string[]
  assignee_ids?: string[]
  linked_event_id?: string | null
}

export type UpdateTaskInput = Partial<Omit<CreateTaskInput, 'board_id'>> & {
  clear_linked_event?: boolean
  clear_color?: boolean
}

export interface UserBrief {
  id: string
  username: string
  display_name: string
  avatar_url: string | null
}

export type Collection = 'today' | 'upcoming' | 'overdue' | 'important' | 'completed' | 'all'

export interface TasksQuery {
  board_id?: string
  stack_id?: string
  status?: string
  collection?: Collection
  due_before?: string
  due_after?: string
  assignee?: string
  label_id?: string
  search?: string
  include_subtasks?: boolean
}

// ── Client ────────────────────────────────────────────────────────────────────

const qs = (q: Record<string, unknown>) => {
  const p = new URLSearchParams()
  for (const [k, v] of Object.entries(q)) {
    if (v !== undefined && v !== null && v !== '') p.set(k, String(v))
  }
  const s = p.toString()
  return s ? `?${s}` : ''
}

export const tasksApi = {
  // Boards
  listBoards: () => apiClient.get<{ boards: Board[] }>('/tasks/boards').then(r => r.data.boards),
  getBoard: (id: string) =>
    apiClient.get<{ board: Board; shares: BoardShare[] }>(`/tasks/boards/${id}`).then(r => r.data),
  createBoard: (body: { title: string; description?: string; color?: string; board_type?: string }) =>
    apiClient.post<{ board: Board }>('/tasks/boards', body).then(r => r.data.board),
  updateBoard: (id: string, body: Partial<Board>) =>
    apiClient.patch<{ board: Board }>(`/tasks/boards/${id}`, body).then(r => r.data.board),
  deleteBoard: (id: string) => apiClient.delete(`/tasks/boards/${id}`).then(() => undefined),
  shareBoard: (id: string, body: { user_id: string; permission?: string }) =>
    apiClient.post(`/tasks/boards/${id}/share`, body).then(r => r.data),
  unshareBoard: (id: string, uid: string) =>
    apiClient.delete(`/tasks/boards/${id}/share/${uid}`).then(() => undefined),
  exportBoardUrl: (id: string) => `/api/v1/tasks/boards/${id}/export`,
  importBoard: (id: string, ics: string) =>
    apiClient.post<{ imported: number }>(`/tasks/boards/${id}/import`, ics, {
      headers: { 'Content-Type': 'text/calendar' },
    }).then(r => r.data),

  // Stacks
  listStacks: (boardId: string) =>
    apiClient.get<{ stacks: Stack[] }>(`/tasks/boards/${boardId}/stacks`).then(r => r.data.stacks),
  createStack: (boardId: string, body: { title: string; sort_order?: number }) =>
    apiClient.post<{ stack: Stack }>(`/tasks/boards/${boardId}/stacks`, body).then(r => r.data.stack),
  updateStack: (id: string, body: { title?: string; sort_order?: number }) =>
    apiClient.patch<{ stack: Stack }>(`/tasks/stacks/${id}`, body).then(r => r.data.stack),
  deleteStack: (id: string) => apiClient.delete(`/tasks/stacks/${id}`).then(() => undefined),
  reorderStacks: (boardId: string, ordered_ids: string[]) =>
    apiClient.post<{ stacks: Stack[] }>(`/tasks/boards/${boardId}/stacks/reorder`, { ordered_ids }).then(r => r.data.stacks),

  // Labels
  listLabels: (boardId: string) =>
    apiClient.get<{ labels: Label[] }>(`/tasks/boards/${boardId}/labels`).then(r => r.data.labels),
  createLabel: (boardId: string, body: { title: string; color?: string }) =>
    apiClient.post<{ label: Label }>(`/tasks/boards/${boardId}/labels`, body).then(r => r.data.label),
  updateLabel: (id: string, body: { title?: string; color?: string }) =>
    apiClient.patch<{ label: Label }>(`/tasks/labels/${id}`, body).then(r => r.data.label),
  deleteLabel: (id: string) => apiClient.delete(`/tasks/labels/${id}`).then(() => undefined),

  // Tasks
  listTasks: (q: TasksQuery = {}) =>
    apiClient.get<{ tasks: Task[] }>(`/tasks/tasks${qs(q as Record<string, unknown>)}`).then(r => r.data.tasks),
  getTask: (id: string) =>
    apiClient.get<{ task: Task }>(`/tasks/tasks/${id}`).then(r => r.data.task),
  createTask: (body: CreateTaskInput) =>
    apiClient.post<{ task: Task }>('/tasks/tasks', body).then(r => r.data.task),
  updateTask: (id: string, body: UpdateTaskInput) =>
    apiClient.patch<{ task: Task }>(`/tasks/tasks/${id}`, body).then(r => r.data.task),
  deleteTask: (id: string) => apiClient.delete(`/tasks/tasks/${id}`).then(() => undefined),
  moveTask: (id: string, body: { stack_id?: string | null; position: number; sort_order?: number }) =>
    apiClient.post<{ task: Task }>(`/tasks/tasks/${id}/move`, body).then(r => r.data.task),
  completeTask: (id: string) =>
    apiClient.post<{ task: Task }>(`/tasks/tasks/${id}/complete`, {}).then(r => r.data.task),
  moveTasksToBoard: (task_ids: string[], target_board_id: string, target_stack_id?: string | null) =>
    apiClient.post<{ moved: string[] }>('/tasks/move-tasks', { task_ids, target_board_id, target_stack_id })
      .then(r => r.data.moved),
  listSubtasks: (id: string) =>
    apiClient.get<{ tasks: Task[] }>(`/tasks/tasks/${id}/subtasks`).then(r => r.data.tasks),
  createSubtask: (id: string, body: CreateTaskInput) =>
    apiClient.post<{ task: Task }>(`/tasks/tasks/${id}/subtasks`, body).then(r => r.data.task),
  addAssignee: (id: string, user_id: string) =>
    apiClient.post(`/tasks/tasks/${id}/assignees`, { user_id }).then(() => undefined),
  removeAssignee: (id: string, uid: string) =>
    apiClient.delete(`/tasks/tasks/${id}/assignees/${uid}`).then(() => undefined),
  addLabel: (id: string, labelId: string) =>
    apiClient.put(`/tasks/tasks/${id}/labels/${labelId}`).then(() => undefined),
  removeLabel: (id: string, labelId: string) =>
    apiClient.delete(`/tasks/tasks/${id}/labels/${labelId}`).then(() => undefined),

  // Users (annuaire du core)
  searchUsers: (q: string) =>
    apiClient.get<{ users: UserBrief[] }>('/users/search', { params: { q, limit: 8 } }).then(r => r.data.users),
  lookupUsers: (ids: string[]) =>
    ids.length === 0
      ? Promise.resolve([] as UserBrief[])
      : apiClient.get<{ users: UserBrief[] }>('/users/lookup', { params: { ids: ids.join(',') } }).then(r => r.data.users),

  // Board comments
  listBoardComments: (boardId: string) =>
    apiClient.get<{ comments: Comment[] }>(`/tasks/boards/${boardId}/comments`).then(r => r.data.comments),
  createBoardComment: (boardId: string, body: string) =>
    apiClient.post<{ comment: Comment }>(`/tasks/boards/${boardId}/comments`, { body }).then(r => r.data.comment),
  updateBoardComment: (id: string, body: string) =>
    apiClient.patch<{ comment: Comment }>(`/tasks/board-comments/${id}`, { body }).then(r => r.data.comment),
  deleteBoardComment: (id: string) => apiClient.delete(`/tasks/board-comments/${id}`).then(() => undefined),

  // Comments
  listComments: (taskId: string) =>
    apiClient.get<{ comments: Comment[] }>(`/tasks/tasks/${taskId}/comments`).then(r => r.data.comments),
  createComment: (taskId: string, body: string) =>
    apiClient.post<{ comment: Comment }>(`/tasks/tasks/${taskId}/comments`, { body }).then(r => r.data.comment),
  updateComment: (id: string, body: string) =>
    apiClient.patch<{ comment: Comment }>(`/tasks/comments/${id}`, { body }).then(r => r.data.comment),
  deleteComment: (id: string) => apiClient.delete(`/tasks/comments/${id}`).then(() => undefined),

  // Attachments
  listAttachments: (taskId: string) =>
    apiClient.get<{ attachments: Attachment[] }>(`/tasks/tasks/${taskId}/attachments`).then(r => r.data.attachments),
  deleteAttachment: (id: string) => apiClient.delete(`/tasks/attachments/${id}`).then(() => undefined),
}
