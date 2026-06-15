import { useState } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import {
  CalendarClock, CalendarDays, AlertTriangle, Star, CheckCircle2, ListTodo, Columns3, Inbox,
  MoreVertical, Pencil, Trash2,
} from 'lucide-react'
import { MenuDropdown, type MenuItem } from '@ui'
import { ConfirmDialog } from '@ui'
import { useConfirm } from '@kubuno/sdk'
import { SidebarNavItem } from '@kubuno/sdk'
import { tasksApi, type Board, type Collection } from './api'
import { useTasksStore } from './store'
import BoardEditWindow from './BoardEditWindow'

const COLLECTIONS: { key: Collection; icon: React.ReactNode }[] = [
  { key: 'today',     icon: <CalendarDays size={18} /> },
  { key: 'upcoming',  icon: <CalendarClock size={18} /> },
  { key: 'overdue',   icon: <AlertTriangle size={18} /> },
  { key: 'important', icon: <Star size={18} /> },
  { key: 'completed', icon: <CheckCircle2 size={18} /> },
  { key: 'all',       icon: <ListTodo size={18} /> },
]

export default function TasksSidebarBody({ collapsed = false }: { collapsed?: boolean }) {
  const { t } = useTranslation('tasks')
  const navigate = useNavigate()
  const qc = useQueryClient()
  const params = useParams()
  const activeBoardId = params.id ?? null
  const collection = useTasksStore(s => s.collection)
  const setCollection = useTasksStore(s => s.setCollection)
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()
  const [menu, setMenu] = useState<{ board: Board; pos: { top: number; left: number } } | null>(null)
  const [editing, setEditing] = useState<Board | null>(null)

  const { data: boards = [] } = useQuery({ queryKey: ['tasks-boards'], queryFn: tasksApi.listBoards })

  const removeBoard = async (board: Board) => {
    if (await confirm({ title: t('delete_board'), message: t('confirm_delete_board'), confirmLabel: t('delete'), variant: 'danger' })) {
      await tasksApi.deleteBoard(board.id)
      qc.invalidateQueries({ queryKey: ['tasks-boards'] })
      if (activeBoardId === board.id) navigate('/tasks')
    }
  }

  const menuItems: MenuItem[] = menu ? [
    { type: 'action', label: t('edit_board'), icon: <Pencil size={15} />, onClick: () => { const b = menu.board; setMenu(null); setEditing(b) } },
    ...(menu.board.is_default ? [] : [
      { type: 'separator' as const },
      { type: 'action' as const, label: t('delete_board'), icon: <Trash2 size={15} />, onClick: () => { const b = menu.board; setMenu(null); removeBoard(b) } },
    ]),
  ] : []

  return (
    <div className="flex flex-col gap-0.5 px-2 py-2">
      {COLLECTIONS.map(c => (
        <SidebarNavItem
          key={c.key}
          label={t(`collection_${c.key}`)}
          icon={c.icon}
          collapsed={collapsed}
          active={!activeBoardId && collection === c.key}
          onClick={() => { setCollection(c.key); navigate('/tasks') }}
        />
      ))}

      {!collapsed && (
        <div className="px-2 pt-3 pb-1 text-[11px] font-semibold uppercase tracking-wide text-text-tertiary">
          {t('boards')}
        </div>
      )}

      {boards.filter(b => !b.is_archived).map(b => (
        <div
          key={b.id}
          className="relative group"
          onContextMenu={(e) => { e.preventDefault(); setMenu({ board: b, pos: { top: e.clientY, left: e.clientX } }) }}
        >
          <SidebarNavItem
            label={b.is_default ? t('default_board') : b.title}
            icon={b.is_default
              ? <Inbox size={18} style={{ color: b.color }} />
              : <Columns3 size={18} style={{ color: b.color }} />}
            collapsed={collapsed}
            to={`/tasks/boards/${b.id}`}
            active={activeBoardId === b.id}
          />
          {/* Menu Modifier/Supprimer (clic gauche sur «…» ou clic droit sur la ligne) */}
          {!collapsed && (
            <button
              onClick={(e) => { e.preventDefault(); e.stopPropagation(); setMenu({ board: b, pos: { top: e.clientY, left: e.clientX } }) }}
              className="absolute right-1.5 top-1/2 -translate-y-1/2 p-1 rounded opacity-0 group-hover:opacity-100
                         text-text-tertiary hover:text-text-primary hover:bg-surface-2 z-10"
              title={t('edit_board')}
            >
              <MoreVertical size={15} />
            </button>
          )}
        </div>
      ))}

      {menu && <MenuDropdown items={menuItems} pos={menu.pos} onClose={() => setMenu(null)} />}
      {editing && (
        <BoardEditWindow
          board={editing}
          onClose={() => setEditing(null)}
          onDeleted={() => { if (activeBoardId === editing.id) navigate('/tasks') }}
        />
      )}
      {confirmState && <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />}
    </div>
  )
}
