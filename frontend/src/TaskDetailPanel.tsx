import { useEffect, useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { Trash2, Plus, Tag, CheckCircle2, Circle, CheckSquare, Check, X } from 'lucide-react'
import { Dropdown, DatePicker, Spinner, Button, Tabs, Input, Textarea } from '@ui'
import { FloatingWindow } from '@ui'
import { ConfirmDialog } from '@ui'
import { useConfirm } from '@kubuno/sdk'
import { tasksApi, type Task, type TaskStatus } from './api'
import { useTasksStore } from './store'
import UserPicker, { UserAvatar } from './UserPicker'
import CommentThread from './CommentThread'
import { useAuthStore } from '@kubuno/sdk'

const STATUS_VALUES: TaskStatus[] = ['open', 'in_progress', 'done', 'cancelled']
const SWATCHES = ['#1a73e8', '#1e8e3e', '#d93025', '#f9ab00', '#9334e6', '#e8710a', '#12b5cb', '#5f6368']

export default function TaskDetailPanel() {
  const { t } = useTranslation('tasks')
  const qc = useQueryClient()
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()
  const authUser = useAuthStore(s => s.user)
  const taskId = useTasksStore(s => s.selectedTaskId)
  const close = () => useTasksStore.getState().selectTask(null)

  const { data: task, isLoading } = useQuery({
    queryKey: ['task-detail', taskId],
    queryFn: () => tasksApi.getTask(taskId!),
    enabled: !!taskId,
  })

  const [title, setTitle] = useState('')
  const [description, setDescription] = useState('')
  const [newSubtask, setNewSubtask] = useState('')
  const [tab, setTab] = useState<'details' | 'subtasks' | 'comments'>('details')

  useEffect(() => {
    if (task) {
      setTitle(task.title)
      setDescription(task.description ?? '')
    }
  }, [task?.id]) // eslint-disable-line react-hooks/exhaustive-deps

  const invalidate = () => {
    qc.invalidateQueries({ queryKey: ['task-detail', taskId] })
    qc.invalidateQueries({ queryKey: ['tasks-board'] })
    qc.invalidateQueries({ queryKey: ['tasks-list'] })
    qc.invalidateQueries({ queryKey: ['tasks-subtasks'] })
  }

  const updateMut = useMutation({
    mutationFn: (body: Parameters<typeof tasksApi.updateTask>[1]) => tasksApi.updateTask(taskId!, body),
    onSuccess: invalidate,
  })
  const deleteMut = useMutation({
    mutationFn: () => tasksApi.deleteTask(taskId!),
    onSuccess: () => { close(); invalidate() },
  })

  const labelsQ = useQuery({
    queryKey: ['board-labels', task?.board_id],
    queryFn: () => tasksApi.listLabels(task!.board_id),
    enabled: !!task?.board_id,
  })
  const commentsQ = useQuery({
    queryKey: ['task-comments', taskId],
    queryFn: () => tasksApi.listComments(taskId!),
    enabled: !!taskId,
  })
  const commentAuthorIds = [...new Set((commentsQ.data ?? []).map(c => c.author_id))]
  const commentAuthorsQ = useQuery({
    queryKey: ['comment-authors', commentAuthorIds],
    queryFn: () => tasksApi.lookupUsers(commentAuthorIds),
    enabled: commentAuthorIds.length > 0,
  })
  const subtasksQ = useQuery({
    queryKey: ['task-detail-subtasks', taskId],
    queryFn: () => tasksApi.listSubtasks(taskId!),
    enabled: !!taskId,
  })
  const boardsQ = useQuery({ queryKey: ['tasks-boards'], queryFn: tasksApi.listBoards })

  const moveToBoardMut = useMutation({
    mutationFn: (bId: string) => tasksApi.moveTasksToBoard([taskId!], bId),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['task-detail', taskId] })
      qc.invalidateQueries({ queryKey: ['tasks-board'] })
      qc.invalidateQueries({ queryKey: ['tasks-list'] })
    },
  })

  const assigneeIds = task?.assignees ?? []
  const assigneeUsersQ = useQuery({
    queryKey: ['assignee-users', assigneeIds],
    queryFn: () => tasksApi.lookupUsers(assigneeIds),
    enabled: assigneeIds.length > 0,
  })
  const addAssigneeMut = useMutation({
    mutationFn: (uid: string) => tasksApi.addAssignee(taskId!, uid),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['task-detail', taskId] })
      qc.invalidateQueries({ queryKey: ['tasks-boards'] }) // l'attribution peut créer un partage
    },
  })
  const removeAssigneeMut = useMutation({
    mutationFn: (uid: string) => tasksApi.removeAssignee(taskId!, uid),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['task-detail', taskId] }),
  })

  const addCommentMut = useMutation({
    mutationFn: (body: string) => tasksApi.createComment(taskId!, body),
    onSuccess: () => commentsQ.refetch(),
  })
  const editCommentMut = useMutation({
    mutationFn: (v: { id: string; body: string }) => tasksApi.updateComment(v.id, v.body),
    onSuccess: () => commentsQ.refetch(),
  })
  const deleteCommentMut = useMutation({
    mutationFn: (id: string) => tasksApi.deleteComment(id),
    onSuccess: () => commentsQ.refetch(),
  })
  const addSubtaskMut = useMutation({
    mutationFn: (titleV: string) => tasksApi.createSubtask(taskId!, { board_id: task!.board_id, title: titleV }),
    onSuccess: () => { setNewSubtask(''); subtasksQ.refetch(); invalidate() },
  })
  const toggleLabelMut = useMutation({
    mutationFn: (v: { labelId: string; on: boolean }) =>
      v.on ? tasksApi.addLabel(taskId!, v.labelId) : tasksApi.removeLabel(taskId!, v.labelId),
    onSuccess: invalidate,
  })
  const toggleSubtaskMut = useMutation({
    mutationFn: (st: Task) =>
      st.status === 'done' ? tasksApi.updateTask(st.id, { status: 'open' }) : tasksApi.completeTask(st.id),
    onSuccess: () => { subtasksQ.refetch(); invalidate() },
  })

  if (!taskId) return null

  const activeLabelIds = new Set((task?.labels ?? []).map(l => l.id))

  return (
    <>
      <FloatingWindow
        title={t('details')}
        icon={<CheckSquare size={18} className="text-primary" />}
        onClose={close}
        backdrop
        resizable
        defaultWidth={460}
        defaultHeight={660}
      >
        {isLoading || !task ? (
          <div className="flex-1 flex items-center justify-center"><Spinner /></div>
        ) : (
          <div className="flex flex-col flex-1 min-h-0">
            {/* Titre — toujours visible */}
            <div className="px-4 pt-3">
              <input
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                onBlur={() => title.trim() && title !== task.title && updateMut.mutate({ title: title.trim() })}
                className="w-full text-lg font-semibold text-text-primary focus:outline-none border-b border-transparent focus:border-border pb-1"
              />
            </div>

            <Tabs
              className="px-3"
              variant="stretched"
              value={tab}
              onChange={setTab}
              tabs={[
                { id: 'details',  label: t('details') },
                { id: 'subtasks', label: t('subtasks'), badge: subtasksQ.data?.length || undefined },
                { id: 'comments', label: t('comments'), badge: commentsQ.data?.length || undefined },
              ]}
            />

            <div className="flex-1 overflow-y-auto p-4 space-y-5">
            {tab === 'details' && (<>
            {/* Tableau (déplacement vers un autre board) */}
            <div>
              <label className="block text-xs text-text-tertiary mb-1">{t('board')}</label>
              <Dropdown
                value={task.board_id}
                onChange={(v) => { if (v !== task.board_id) moveToBoardMut.mutate(v) }}
                width="100%"
                options={(boardsQ.data ?? []).filter(b => !b.is_archived).map(b => ({
                  value: b.id,
                  label: b.is_default ? t('default_board') : b.title,
                }))}
              />
            </div>

            {/* Statut + priorité */}
            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="block text-xs text-text-tertiary mb-1">{t('status')}</label>
                <Dropdown
                  value={task.status}
                  onChange={(v) => updateMut.mutate({ status: v as TaskStatus })}
                  width="100%"
                  options={STATUS_VALUES.map(s => ({ value: s, label: t(`status_${s}`) }))}
                />
              </div>
              <div>
                <label className="block text-xs text-text-tertiary mb-1">{t('priority')}</label>
                <Dropdown
                  value={String(task.priority)}
                  onChange={(v) => updateMut.mutate({ priority: Number(v) })}
                  width="100%"
                  options={[
                    { value: '0', label: t('priority_none') },
                    { value: '1', label: t('priority_high') },
                    { value: '5', label: t('priority_medium') },
                    { value: '9', label: t('priority_low') },
                  ]}
                />
              </div>
            </div>

            {/* Échéance + avancement */}
            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="block text-xs text-text-tertiary mb-1">{t('due_date')}</label>
                <DatePicker
                  mode="datetime"
                  value={task.due_at}
                  onChange={(v) => updateMut.mutate({ due_at: v ? new Date(v).toISOString() : null })}
                  clearable
                />
              </div>
              <div>
                <label className="block text-xs text-text-tertiary mb-1">{t('percent_complete')}</label>
                <input
                  type="range" min={0} max={100} step={5}
                  value={task.percent_complete}
                  onChange={(e) => updateMut.mutate({ percent_complete: Number(e.target.value) })}
                  className="w-full"
                />
                <span className="text-xs text-text-tertiary">{task.percent_complete}%</span>
              </div>
            </div>

            {/* Description */}
            <div>
              <label className="block text-xs text-text-tertiary mb-1">{t('description')}</label>
              <Textarea
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                onBlur={() => description !== (task.description ?? '') && updateMut.mutate({ description })}
                rows={4}
                placeholder={t('description_ph')}
                className="h-auto min-h-0"
              />
            </div>

            {/* Couleur (hérite du board par défaut) */}
            <div>
              <label className="block text-xs text-text-tertiary mb-1">{t('color')}</label>
              <div className="flex flex-wrap items-center gap-2">
                {/* Par défaut = couleur du board */}
                <button
                  type="button"
                  onClick={() => updateMut.mutate({ clear_color: true })}
                  title={t('color_default')}
                  className="h-7 px-2 rounded-full flex items-center gap-1.5 border text-xs transition"
                  style={!task.color
                    ? { borderColor: 'var(--color-primary, #1a73e8)', color: '#1a73e8' }
                    : { borderColor: 'var(--color-border, #dadce0)' }}
                >
                  <span className="w-3.5 h-3.5 rounded-full" style={{ backgroundColor: (boardsQ.data ?? []).find(b => b.id === task.board_id)?.color ?? '#1a73e8' }} />
                  {t('color_default')}
                </button>
                {SWATCHES.map(c => (
                  <button
                    key={c}
                    type="button"
                    onClick={() => updateMut.mutate({ color: c })}
                    className="w-7 h-7 rounded-full flex items-center justify-center transition-transform hover:scale-110"
                    style={{ backgroundColor: c, outline: task.color === c ? '2px solid #1a73e8' : 'none', outlineOffset: 2 }}
                  >
                    {task.color === c && <Check size={14} className="text-white" />}
                  </button>
                ))}
              </div>
            </div>

            {/* Labels */}
            <div>
              <label className="block text-xs text-text-tertiary mb-1 flex items-center gap-1"><Tag size={12} />{t('labels')}</label>
              <div className="flex flex-wrap gap-1.5">
                {(labelsQ.data ?? []).map(l => {
                  const on = activeLabelIds.has(l.id)
                  return (
                    <button
                      key={l.id}
                      onClick={() => toggleLabelMut.mutate({ labelId: l.id, on: !on })}
                      className={`text-xs px-2 py-0.5 rounded-full border transition ${on ? 'text-white border-transparent' : 'text-text-secondary border-border'}`}
                      style={on ? { backgroundColor: l.color } : undefined}
                    >
                      {l.title}
                    </button>
                  )
                })}
                {(labelsQ.data ?? []).length === 0 && (
                  <span className="text-xs text-text-tertiary">{t('no_labels')}</span>
                )}
              </div>
            </div>

            {/* Assignés (l'attribution partage naturellement le board) */}
            <div>
              <label className="block text-xs text-text-tertiary mb-1">{t('assignees')}</label>
              <div className="space-y-1.5 mb-2">
                {(assigneeUsersQ.data ?? []).map(u => (
                  <div key={u.id} className="flex items-center gap-2">
                    <UserAvatar user={u} size={22} />
                    <span className="flex-1 text-sm text-text-primary truncate">{u.display_name}</span>
                    <button
                      onClick={() => removeAssigneeMut.mutate(u.id)}
                      className="text-text-tertiary hover:text-danger p-0.5"
                      title={t('remove')}
                    ><X size={14} /></button>
                  </div>
                ))}
                {assigneeIds.length === 0 && (
                  <span className="text-xs text-text-tertiary">{t('no_assignees')}</span>
                )}
              </div>
              <UserPicker
                placeholder={t('assign_placeholder')}
                excludeIds={assigneeIds}
                onPick={(u) => addAssigneeMut.mutate(u.id)}
              />
            </div>

            </>)}

            {tab === 'subtasks' && (<>
            {/* Sous-tâches */}
            <div>
              <label className="block text-xs text-text-tertiary mb-1">{t('subtasks')}</label>
              <div className="space-y-1">
                {(subtasksQ.data ?? []).map(st => (
                  <div key={st.id} className="flex items-center gap-2 text-sm">
                    <button onClick={() => toggleSubtaskMut.mutate(st)} className="text-text-tertiary hover:text-success">
                      {st.status === 'done' ? <CheckCircle2 size={15} className="text-success" /> : <Circle size={15} />}
                    </button>
                    <span className={st.status === 'done' ? 'line-through text-text-tertiary' : 'text-text-primary'}>{st.title}</span>
                  </div>
                ))}
              </div>
              <div className="flex items-center gap-2 mt-2">
                <div className="flex-1">
                  <Input
                    value={newSubtask}
                    onChange={(e) => setNewSubtask(e.target.value)}
                    onKeyDown={(e) => { if (e.key === 'Enter' && newSubtask.trim()) addSubtaskMut.mutate(newSubtask.trim()) }}
                    placeholder={t('add_subtask')}
                  />
                </div>
                <button onClick={() => newSubtask.trim() && addSubtaskMut.mutate(newSubtask.trim())} className="text-text-secondary hover:text-primary px-1"><Plus size={18} /></button>
              </div>
            </div>

            </>)}

            {tab === 'comments' && (
              <CommentThread
                comments={commentsQ.data ?? []}
                authors={commentAuthorsQ.data ?? []}
                currentUserId={authUser?.id}
                onAdd={(body) => addCommentMut.mutate(body)}
                onEdit={(id, body) => editCommentMut.mutate({ id, body })}
                onDelete={(id) => deleteCommentMut.mutate(id)}
              />
            )}
            </div>
          </div>
        )}

        <div className="border-t border-border p-3 flex-shrink-0">
          <Button
            variant="ghost"
            onClick={async () => {
              if (await confirm({ title: t('delete_task'), message: t('confirm_delete_task'), confirmLabel: t('delete'), variant: 'danger' })) {
                deleteMut.mutate()
              }
            }}
            className="text-danger w-full flex items-center justify-center gap-2"
          >
            <Trash2 size={16} /> {t('delete_task')}
          </Button>
        </div>
      </FloatingWindow>

      {confirmState && (
        <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />
      )}
    </>
  )
}
