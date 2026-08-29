import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Dropdown, Button, Input } from '@ui'
import { useSearchStore } from '@kubuno/sdk'
import { useTasksStore } from './store'
import { STATUS_ORDER } from './helpers'

export default function TasksFilterPanel({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation('tasks')
  const filters = useTasksStore(s => s.filters)
  const setFilters = useTasksStore(s => s.setFilters)
  const clearFilters = useTasksStore(s => s.clearFilters)
  const setSearchQuery = useTasksStore(s => s.setSearchQuery)

  // ── Two-way sync with the shell search bar (platform rule) ──────────────────
  // Tasks' search has no text operators: the bar's query is plain free text
  // (backend `search` parameter). The « Contains the words » field below
  // mirrors it: opening the panel pre-fills it with the current query, and
  // editing it rewrites the bar's text live (running the search like typing
  // does). The status dropdown is a state filter with no query-text
  // representation — it deliberately stays panel-only (no invented operators).
  const query    = useSearchStore(s => s.query)
  const setQuery = useSearchStore(s => s.setQuery)
  const [words, setWords] = useState(query)
  // Remembers the last query WE pushed so its echo doesn't clobber the field.
  const lastBuilt = useRef<string | null>(null)
  useEffect(() => {
    if (query === lastBuilt.current) return
    setWords(query)
  }, [query])

  const setContainsWords = (v: string) => {
    setWords(v)
    lastBuilt.current = v
    setQuery(v)          // rewrite the bar's text live
    setSearchQuery(v)    // run the live search, like typing in the bar does
  }

  const handleReset = () => {
    clearFilters()
    setWords('')
    lastBuilt.current = ''
    setQuery('')
    setSearchQuery('')
  }

  return (
    <div className="p-3 w-64 space-y-3">
      <div>
        <label className="block text-xs text-text-tertiary mb-1">{t('filter_words')}</label>
        <Input
          type="text"
          placeholder={t('search_ph')}
          value={words}
          onChange={e => setContainsWords(e.target.value)}
        />
      </div>
      <div>
        <label className="block text-xs text-text-tertiary mb-1">{t('status')}</label>
        <Dropdown
          width="100%"
          value={filters.status ?? ''}
          onChange={(v) => setFilters({ status: v || null })}
          options={[{ value: '', label: t('all') }, ...STATUS_ORDER.map(s => ({ value: s, label: t(`status_${s}`) }))]}
        />
      </div>
      <div className="flex justify-between items-center">
        <button onClick={handleReset} className="text-xs text-text-secondary hover:text-text-primary">{t('filter_reset')}</button>
        <Button size="sm" onClick={onClose}>{t('filter_apply')}</Button>
      </div>
    </div>
  )
}
