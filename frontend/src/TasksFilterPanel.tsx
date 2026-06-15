import { useTranslation } from 'react-i18next'
import { Dropdown, Button } from '@ui'
import { useTasksStore } from './store'
import { STATUS_ORDER } from './helpers'

export default function TasksFilterPanel({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation('tasks')
  const filters = useTasksStore(s => s.filters)
  const setFilters = useTasksStore(s => s.setFilters)
  const clearFilters = useTasksStore(s => s.clearFilters)

  return (
    <div className="p-3 w-64 space-y-3">
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
        <button onClick={() => { clearFilters() }} className="text-xs text-text-secondary hover:text-text-primary">{t('filter_reset')}</button>
        <Button size="sm" onClick={onClose}>{t('filter_apply')}</Button>
      </div>
    </div>
  )
}
