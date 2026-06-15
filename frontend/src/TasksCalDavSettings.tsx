import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Copy, Check, Columns3 } from 'lucide-react'
import { useAuthStore } from '@kubuno/sdk'
import { tasksApi } from './api'

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false)
  return (
    <button
      onClick={() => { navigator.clipboard.writeText(text); setCopied(true); setTimeout(() => setCopied(false), 2000) }}
      className="flex-shrink-0 p-1.5 rounded hover:bg-surface-2 text-text-tertiary hover:text-text-primary"
    >
      {copied ? <Check size={14} className="text-green-600" /> : <Copy size={14} />}
    </button>
  )
}

export default function TasksCalDavSettings() {
  const { t } = useTranslation('tasks')
  const user = useAuthStore(s => s.user)
  const { data: boards = [] } = useQuery({ queryKey: ['tasks-boards'], queryFn: tasksApi.listBoards })

  const username = user?.username ?? 'user'
  const origin = typeof window !== 'undefined' ? window.location.origin : ''

  return (
    <section className="space-y-3">
      <div>
        <h3 className="text-sm font-semibold text-text-primary">{t('caldav_title')}</h3>
        <p className="text-xs text-text-secondary mt-0.5">{t('caldav_help')}</p>
      </div>

      <div className="space-y-2">
        {boards.filter(b => !b.is_archived).map(b => {
          const url = `${origin}/api/v1/tasks/caldav/${username}/${b.caldav_token}/`
          return (
            <div key={b.id} className="flex items-center gap-2 border border-border rounded-lg px-3 py-2">
              <Columns3 size={16} style={{ color: b.color }} className="flex-shrink-0" />
              <span className="text-sm text-text-primary w-32 truncate flex-shrink-0">{b.title}</span>
              <span className="flex-1 text-xs font-mono bg-surface-2 rounded px-2 py-0.5 truncate min-w-0">{url}</span>
              <CopyButton text={url} />
            </div>
          )
        })}
        {boards.length === 0 && <p className="text-xs text-text-tertiary">{t('no_boards')}</p>}
      </div>
    </section>
  )
}
