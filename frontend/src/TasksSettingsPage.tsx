import { useTranslation } from 'react-i18next'
import TasksCalDavSettings from './TasksCalDavSettings'

export default function TasksSettingsPage() {
  const { t } = useTranslation('tasks')
  return (
    <div className="h-full overflow-y-auto">
      <div className="max-w-2xl mx-auto py-6 px-4 space-y-6">
        <h1 className="text-xl font-semibold text-text-primary">{t('settings_title')}</h1>
        <TasksCalDavSettings />
      </div>
    </div>
  )
}
