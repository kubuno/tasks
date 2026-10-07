import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Pencil, Trash2, X } from 'lucide-react'
import type { Comment, UserBrief } from './api'
import { UserAvatar } from './UserPicker'
import CommentBody from './CommentBody'
import CommentEditor from './CommentEditor'

interface Props {
  comments: Comment[]
  authors: UserBrief[]
  currentUserId?: string
  onAdd: (body: string) => void
  onEdit: (id: string, body: string) => void
  onDelete: (id: string) => void
}

export default function CommentThread({ comments, authors, currentUserId, onAdd, onEdit, onDelete }: Props) {
  const { t } = useTranslation('tasks')
  const [draft, setDraft] = useState('')
  const [editingId, setEditingId] = useState<string | null>(null)
  const [editDraft, setEditDraft] = useState('')

  // Seul le DERNIER commentaire est éditable/supprimable, et uniquement par son auteur.
  const lastId = comments.length > 0 ? comments[comments.length - 1].id : null
  const editing = editingId !== null

  const startEdit = (c: Comment) => { setEditingId(c.id); setEditDraft(c.body) }
  const cancelEdit = () => { setEditingId(null); setEditDraft('') }

  // L'éditeur du bas sert à la fois à AJOUTER et à MODIFIER (mode édition).
  const value = editing ? editDraft : draft
  const setValue = editing ? setEditDraft : setDraft
  const submit = () => {
    if (editing) {
      if (editDraft.trim()) { onEdit(editingId!, editDraft.trim()); cancelEdit() }
    } else {
      if (draft.trim()) { onAdd(draft.trim()); setDraft('') }
    }
  }

  return (
    <div className="flex flex-col h-full">
      <div className="flex-1 space-y-2 mb-2">
        {comments.map(c => {
          const author = authors.find(u => u.id === c.author_id)
          const canEdit = c.id === lastId && c.author_id === currentUserId
          const beingEdited = c.id === editingId
          return (
            <div key={c.id}
              className={`text-sm rounded-lg p-2 transition-colors ${beingEdited ? 'bg-primary-light ring-1 ring-primary/40' : 'bg-surface-1'}`}>
              <div className="flex items-center gap-1.5 mb-1">
                <UserAvatar user={author} size={18} />
                <span className="text-xs font-medium text-text-secondary">{author?.display_name ?? '—'}</span>
                <span className="text-[10px] text-text-tertiary ml-auto">{new Date(c.created_at).toLocaleString()}</span>
                {canEdit && (
                  <div className="flex items-center gap-0.5">
                    {beingEdited ? (
                      <button onClick={cancelEdit} className="text-text-tertiary hover:text-text-primary p-0.5" title={t('cancel')}><X size={13} /></button>
                    ) : (
                      <button onClick={() => startEdit(c)} className="text-text-tertiary hover:text-text-primary p-0.5" title={t('edit')}><Pencil size={13} /></button>
                    )}
                    <button onClick={() => onDelete(c.id)} className="text-text-tertiary hover:text-danger p-0.5" title={t('delete')}><Trash2 size={13} /></button>
                  </div>
                )}
              </div>
              <CommentBody body={c.body} />
            </div>
          )
        })}
        {comments.length === 0 && <span className="text-xs text-text-tertiary">{t('no_comments')}</span>}
      </div>

      {editing && (
        <div className="flex items-center justify-between text-xs text-primary mb-1 px-1">
          <span>{t('editing_comment')}</span>
          <button onClick={cancelEdit} className="text-text-secondary hover:text-text-primary">{t('cancel')}</button>
        </div>
      )}
      <CommentEditor value={value} onChange={setValue} onSubmit={submit} placeholder={t('write_comment')} />
    </div>
  )
}
