import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { Columns3, Inbox, Trash2, Check, Copy, X } from 'lucide-react'
import { FloatingWindow } from '@ui'
import { Input, Dropdown, Toggle, Button, Tabs } from '@ui'
import { ConfirmDialog } from '@ui'
import { useConfirm } from '@kubuno/sdk'
import { useAuthStore } from '@kubuno/sdk'
import { tasksApi, type Board } from './api'
import UserPicker, { UserAvatar } from './UserPicker'
import CommentThread from './CommentThread'

const SWATCHES = ['#1a73e8', '#1e8e3e', '#d93025', '#f9ab00', '#9334e6', '#e8710a', '#12b5cb', '#5f6368']

interface Props {
  board: Board
  onClose: () => void
  onDeleted?: () => void
}

type Tab = 'general' | 'share' | 'comments' | 'caldav'

export default function BoardEditWindow({ board, onClose, onDeleted }: Props) {
  const { t } = useTranslation('tasks')
  const qc = useQueryClient()
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()
  const user = useAuthStore(s => s.user)

  const [tab, setTab] = useState<Tab>('general')
  const [title, setTitle] = useState(board.title)
  const [color, setColor] = useState(board.color)
  const [boardType, setBoardType] = useState(board.board_type)
  const [archived, setArchived] = useState(board.is_archived)
  const [copied, setCopied] = useState(false)

  const invalidate = () => qc.invalidateQueries({ queryKey: ['tasks-boards'] })

  // ── Partage ──
  const detailQ = useQuery({ queryKey: ['board-detail', board.id], queryFn: () => tasksApi.getBoard(board.id) })
  const shares = detailQ.data?.shares ?? []
  const shareUsersQ = useQuery({
    queryKey: ['share-users', shares.map(s => s.shared_with)],
    queryFn: () => tasksApi.lookupUsers(shares.map(s => s.shared_with)),
    enabled: shares.length > 0,
  })
  const addShareMut = useMutation({
    mutationFn: (uid: string) => tasksApi.shareBoard(board.id, { user_id: uid, permission: 'write' }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['board-detail', board.id] }),
  })
  const removeShareMut = useMutation({
    mutationFn: (uid: string) => tasksApi.unshareBoard(board.id, uid),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['board-detail', board.id] }),
  })

  // ── Commentaires ──
  const commentsQ = useQuery({ queryKey: ['board-comments', board.id], queryFn: () => tasksApi.listBoardComments(board.id) })
  const commentAuthorIds = [...new Set((commentsQ.data ?? []).map(c => c.author_id))]
  const commentAuthorsQ = useQuery({
    queryKey: ['board-comment-authors', commentAuthorIds],
    queryFn: () => tasksApi.lookupUsers(commentAuthorIds),
    enabled: commentAuthorIds.length > 0,
  })
  const addCommentMut = useMutation({
    mutationFn: (body: string) => tasksApi.createBoardComment(board.id, body),
    onSuccess: () => commentsQ.refetch(),
  })
  const editCommentMut = useMutation({
    mutationFn: (v: { id: string; body: string }) => tasksApi.updateBoardComment(v.id, v.body),
    onSuccess: () => commentsQ.refetch(),
  })
  const deleteCommentMut = useMutation({
    mutationFn: (id: string) => tasksApi.deleteBoardComment(id),
    onSuccess: () => commentsQ.refetch(),
  })

  const saveMut = useMutation({
    mutationFn: () => tasksApi.updateBoard(board.id, {
      title: board.is_default ? undefined : title.trim(),
      color, board_type: boardType,
      is_archived: board.is_default ? undefined : archived,
    } as Partial<Board>),
    onSuccess: () => { invalidate(); qc.invalidateQueries({ queryKey: ['tasks-board-meta', board.id] }); onClose() },
  })
  const deleteMut = useMutation({
    mutationFn: () => tasksApi.deleteBoard(board.id),
    onSuccess: () => { invalidate(); onClose(); onDeleted?.() },
  })

  const caldavUrl = `${typeof window !== 'undefined' ? window.location.origin : ''}/api/v1/tasks/caldav/${user?.username ?? 'user'}/${board.caldav_token}/`

  const tabs: { id: Tab; label: string }[] = [
    { id: 'general', label: t('general') },
    ...(board.is_default ? [] : [{ id: 'share' as Tab, label: t('share') }]),
    { id: 'comments', label: t('comments') },
    { id: 'caldav', label: 'CalDAV' },
  ]

  return (
    <>
      <FloatingWindow
        title={board.is_default ? t('default_board') : t('edit_board')}
        icon={board.is_default ? <Inbox size={18} /> : <Columns3 size={18} />}
        onClose={onClose}
        backdrop
        resizable
        defaultWidth={460}
        defaultHeight={560}
      >
        <Tabs className="px-3 pt-1" variant="stretched" value={tab} onChange={(v) => setTab(v as Tab)} tabs={tabs} />

        <div className="p-4 space-y-4 overflow-y-auto flex-1">
          {tab === 'general' && (
            <>
              <div>
                <label className="block text-xs text-text-tertiary mb-1">{t('title')}</label>
                <Input value={board.is_default ? t('default_board') : title} onChange={(e) => setTitle(e.target.value)} disabled={board.is_default} />
              </div>
              <div>
                <label className="block text-xs text-text-tertiary mb-1">{t('board_type')}</label>
                <Dropdown value={boardType} onChange={(v) => setBoardType(v as 'kanban' | 'list')} width="100%"
                  options={[{ value: 'kanban', label: t('type_kanban') }, { value: 'list', label: t('type_list') }]} />
              </div>
              <div>
                <label className="block text-xs text-text-tertiary mb-1">{t('color')}</label>
                <div className="flex flex-wrap gap-2">
                  {SWATCHES.map(c => (
                    <button key={c} type="button" onClick={() => setColor(c)}
                      className="w-7 h-7 rounded-full flex items-center justify-center transition-transform hover:scale-110"
                      style={{ backgroundColor: c, outline: color === c ? '2px solid #1a73e8' : 'none', outlineOffset: 2 }}>
                      {color === c && <Check size={14} className="text-white" />}
                    </button>
                  ))}
                </div>
              </div>
              {!board.is_default && (
                <div className="flex items-center justify-between">
                  <span className="text-sm text-text-primary">{t('archive')}</span>
                  <Toggle checked={archived} onChange={(e) => setArchived(e.target.checked)} />
                </div>
              )}
            </>
          )}

          {tab === 'share' && (
            <div className="space-y-2">
              {shares.map(s => {
                const u = shareUsersQ.data?.find(x => x.id === s.shared_with)
                return (
                  <div key={s.id} className="flex items-center gap-2">
                    <UserAvatar user={u} size={24} />
                    <span className="flex-1 text-sm text-text-primary truncate">{u?.display_name ?? s.shared_with}</span>
                    <span className="text-[10px] uppercase tracking-wide text-text-tertiary">{s.permission}</span>
                    <button onClick={() => removeShareMut.mutate(s.shared_with)} className="text-text-tertiary hover:text-danger p-0.5" title={t('remove')}><X size={15} /></button>
                  </div>
                )
              })}
              {shares.length === 0 && <p className="text-xs text-text-tertiary">{t('no_shares')}</p>}
              <UserPicker
                placeholder={t('assign_placeholder')}
                excludeIds={[...shares.map(s => s.shared_with), user?.id ?? '']}
                onPick={(u) => addShareMut.mutate(u.id)}
              />
            </div>
          )}

          {tab === 'comments' && (
            <CommentThread
              comments={commentsQ.data ?? []}
              authors={commentAuthorsQ.data ?? []}
              currentUserId={user?.id}
              onAdd={(body) => addCommentMut.mutate(body)}
              onEdit={(id, body) => editCommentMut.mutate({ id, body })}
              onDelete={(id) => deleteCommentMut.mutate(id)}
            />
          )}

          {tab === 'caldav' && (
            <div className="space-y-3">
              <p className="text-xs text-text-secondary">{t('caldav_help')}</p>
              <div className="flex items-center gap-2 border border-border rounded-lg px-3 py-2">
                <span className="flex-1 text-xs font-mono bg-surface-2 rounded px-2 py-1 truncate min-w-0">{caldavUrl}</span>
                <button onClick={() => { navigator.clipboard.writeText(caldavUrl); setCopied(true); setTimeout(() => setCopied(false), 2000) }}
                  className="flex-shrink-0 p-1.5 rounded hover:bg-surface-2 text-text-tertiary hover:text-text-primary">
                  {copied ? <Check size={14} className="text-green-600" /> : <Copy size={14} />}
                </button>
              </div>
            </div>
          )}
        </div>

        <div className="flex items-center justify-between gap-2 px-4 py-3 border-t border-border flex-shrink-0">
          {!board.is_default ? (
            <Button variant="ghost"
              onClick={async () => { if (await confirm({ title: t('delete_board'), message: t('confirm_delete_board'), confirmLabel: t('delete'), variant: 'danger' })) deleteMut.mutate() }}
              className="text-danger flex items-center gap-1.5">
              <Trash2 size={16} /> {t('delete')}
            </Button>
          ) : <span />}
          <div className="flex items-center gap-2">
            <Button variant="ghost" onClick={onClose}>{t('cancel')}</Button>
            <Button variant="primary" onClick={() => saveMut.mutate()}>{t('save')}</Button>
          </div>
        </div>
      </FloatingWindow>

      {confirmState && <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />}
    </>
  )
}
