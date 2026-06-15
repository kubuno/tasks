import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Input } from '@ui'
import { tasksApi, type UserBrief } from './api'

export function UserAvatar({ user, size = 24 }: { user?: UserBrief; size?: number }) {
  const label = user?.display_name ?? user?.username ?? '?'
  const initials = label.trim().slice(0, 2).toUpperCase()
  if (user?.avatar_url) {
    return <img src={user.avatar_url} alt={label} width={size} height={size} className="rounded-full object-cover flex-shrink-0" style={{ width: size, height: size }} />
  }
  return (
    <span
      className="rounded-full flex items-center justify-center bg-primary/15 text-primary font-medium flex-shrink-0"
      style={{ width: size, height: size, fontSize: size * 0.4 }}
      title={label}
    >
      {initials}
    </span>
  )
}

interface Props {
  placeholder: string
  excludeIds?: string[]
  onPick: (user: UserBrief) => void
}

export default function UserPicker({ placeholder, excludeIds = [], onPick }: Props) {
  const [q, setQ] = useState('')
  const [open, setOpen] = useState(false)

  const { data: users = [] } = useQuery({
    queryKey: ['users-search', q],
    queryFn: () => tasksApi.searchUsers(q),
    enabled: q.trim().length >= 1,
  })

  const results = users.filter(u => !excludeIds.includes(u.id))

  return (
    <div className="relative">
      <Input
        value={q}
        onChange={(e) => { setQ(e.target.value); setOpen(true) }}
        onFocus={() => setOpen(true)}
        onBlur={() => setTimeout(() => setOpen(false), 150)}
        placeholder={placeholder}
      />
      {open && q.trim().length >= 1 && results.length > 0 && (
        <div className="absolute z-30 left-0 right-0 mt-1 bg-white rounded-lg border border-border shadow-lg max-h-56 overflow-y-auto">
          {results.map(u => (
            <button
              key={u.id}
              type="button"
              onMouseDown={(e) => { e.preventDefault(); onPick(u); setQ(''); setOpen(false) }}
              className="flex items-center gap-2 w-full px-2.5 py-1.5 text-left text-sm hover:bg-surface-1"
            >
              <UserAvatar user={u} size={22} />
              <span className="min-w-0 flex-1 truncate text-text-primary">{u.display_name}</span>
              <span className="text-xs text-text-tertiary truncate">@{u.username}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  )
}
