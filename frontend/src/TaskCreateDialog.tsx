/**
 * Globally-mounted task creator (slot `app-dialogs`).
 *
 * Rendered by the host shell in every route, so a CONSUMER module (chat…) can
 * create a task purely through `ModuleServiceRegistry.call('tasks', 'createTask')`
 * — no cross-module import, no route change. The pending promise lives in
 * `taskCreateStore` (contacts' `contactPickerStore` pattern) and is settled here.
 */
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { CheckSquare, Loader2 } from 'lucide-react'
import { Button, Dropdown, FloatingWindow, Input, Textarea } from '@ui'
import { tasksApi, type Board } from './api'
import { useTaskCreateStore, type CreateTaskOptions, type CreatedTask } from './taskCreateStore'

interface Props {
  opts: CreateTaskOptions
  onClose: (task: CreatedTask | null) => void
}

function TaskCreateInner({ opts, onClose }: Props) {
  const { t } = useTranslation('tasks')
  const [title, setTitle]       = useState(opts.title ?? '')
  const [description, setDescription] = useState('')
  const [boards, setBoards]     = useState<Board[]>([])
  const [boardId, setBoardId]   = useState('')
  const [loading, setLoading]   = useState(true)
  const [saving, setSaving]     = useState(false)
  const [error, setError]       = useState<string | null>(null)

  // Target boards: the user's own lists/boards. Default = the board flagged
  // `is_default` (created by the backend on first use), else the first one.
  useEffect(() => {
    let cancelled = false
    tasksApi.listBoards()
      .then(list => {
        if (cancelled) return
        const usable = list.filter(b => !b.is_archived)
        setBoards(usable)
        const def = usable.find(b => b.is_default) ?? usable[0]
        if (def) setBoardId(def.id)
      })
      .catch(() => { if (!cancelled) setBoards([]) })
      .finally(() => { if (!cancelled) setLoading(false) })
    return () => { cancelled = true }
  }, [])

  const canSubmit = !saving && !loading && !!boardId && title.trim().length > 0

  const submit = () => {
    if (!canSubmit) return
    setSaving(true)
    setError(null)
    tasksApi.createTask({
      board_id:    boardId,
      title:       title.trim(),
      description: description.trim() || null,
    })
      .then(task => onClose({ id: task.id, title: task.title }))
      .catch(() => {
        setSaving(false)
        setError(t('create_task_error', { defaultValue: 'La tâche n’a pas pu être créée.' }))
      })
  }

  return (
    <FloatingWindow
      title={t('create_task_title', { defaultValue: 'Nouvelle tâche' })}
      icon={<CheckSquare size={17} className="text-primary" />}
      onClose={() => onClose(null)}
      defaultWidth={460}
      defaultHeight={400}
      resizable
    >
      <div className="flex flex-col flex-1 min-h-0">
        <div className="flex-1 overflow-y-auto px-5 py-4 space-y-4">
          <Input
            autoFocus
            label={t('title', { defaultValue: 'Titre' })}
            value={title}
            onChange={e => setTitle(e.target.value)}
            onKeyDown={e => { if (e.key === 'Enter') { e.preventDefault(); submit() } }}
            placeholder={t('create_task_ph', { defaultValue: 'Que faut-il faire ?' })}
            className="w-full"
          />

          <div className="space-y-1.5">
            <label className="block text-xs font-medium text-text-secondary">
              {t('board', { defaultValue: 'Tableau' })}
            </label>
            {loading ? (
              <div className="flex items-center gap-2 text-text-tertiary text-xs h-7">
                <Loader2 size={14} className="animate-spin" />
                {t('loading', { defaultValue: 'Chargement…' })}
              </div>
            ) : boards.length === 0 ? (
              <p className="text-xs text-text-tertiary">
                {t('no_boards', { defaultValue: 'Aucun tableau disponible' })}
              </p>
            ) : (
              <Dropdown
                value={boardId}
                onChange={setBoardId}
                options={boards.map(b => ({ value: b.id, label: b.title }))}
                width="100%"
                height={32}
              />
            )}
          </div>

          <Textarea
            label={t('description', { defaultValue: 'Description' })}
            value={description}
            onChange={e => setDescription(e.target.value)}
            rows={4}
            className="w-full"
          />

          {error && <p className="text-xs text-danger">{error}</p>}
        </div>

        <div className="flex items-center justify-end gap-3 px-5 py-3 border-t border-border bg-surface-1 flex-shrink-0">
          <Button variant="secondary" size="sm" onClick={() => onClose(null)} disabled={saving}>
            {t('cancel', { defaultValue: 'Annuler' })}
          </Button>
          <Button size="sm" onClick={submit} disabled={!canSubmit}>
            {saving
              ? <Loader2 size={14} className="animate-spin" />
              : t('create_task_submit', { defaultValue: 'Créer' })}
          </Button>
        </div>
      </div>
    </FloatingWindow>
  )
}

export default function TaskCreateDialog() {
  const opts    = useTaskCreateStore(s => s.createOpts)
  const resolve = useTaskCreateStore(s => s._resolve)

  if (!opts) return null

  // Remount on each open so the form starts from the caller's options.
  return <TaskCreateInner opts={opts} onClose={resolve} />
}
