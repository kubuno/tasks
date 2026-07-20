/** Bundle MODULE tasks — chargé à l'exécution (cf. vite.module.config). */
import { lazy } from 'react'
import { format, parseISO } from 'date-fns'
import {
  ExtensionRegistry,
  CALENDAR_OVERLAY, type CalendarOverlayItem, type CalendarOverlayProvider,
  RouteRegistry, WidgetRegistry, WaffleAppRegistry,
  ModuleSettingsRegistry,
  ModuleServiceRegistry,
  NotificationRegistry,
  SlotRegistry,
  useSidebarStore, useToolbarStore, useSearchStore, useRightPanelStore,
  SDK_VERSION,
} from '@kubuno/sdk'
import { CheckSquare } from 'lucide-react'
import './index.css'
import './i18n'
import { tasksApi } from './api'
import { useTasksStore } from './store'
import TasksCreateMenu from './TasksCreateMenu'
import TasksSidebarBody from './TasksSidebarBody'
import TasksToolbar from './TasksToolbar'
import TasksMiniPanel from './TasksMiniPanel'
import TasksFilterPanel from './TasksFilterPanel'
import TasksDueWidget from './TasksDueWidget'
import TasksDataCard from './TasksDataCard'
import TaskCreateDialog from './TaskCreateDialog'
import { createTask } from './taskCreateStore'
import { registerDataCardRenderer } from './kubunoData'

export const sdkVersion = SDK_VERSION

export function register() {
  // Task creator, mounted globally by the host shell: any module can open it
  // without navigating to /tasks.
  SlotRegistry.register('app-dialogs', 'tasks', TaskCreateDialog)

  // Services tasks offers to OTHER modules (chat…). Published only while tasks
  // is installed+active → consumers degrade gracefully when absent.
  //   createTask(opts?: { title?: string }): Promise<{ id, title } | null>
  ModuleServiceRegistry.publish('tasks', {
    createTask,
  })

  // `tasks.task` JSON envelopes ("Copier pour Kubuno" in the task menu):
  // consumer modules (chat, notes…) resolve this card through `core.data-card`.
  registerDataCardRenderer('tasks', {
    types: ['tasks.task'],
    Component: TasksDataCard,
  })

  WaffleAppRegistry.register('tasks', 'Tasks', [
    { id: 'tasks', label: 'Tasks', Icon: CheckSquare, path: '/tasks' },
  ])

  // The header gear button opens the per-user Tasks settings while in /tasks
  // (the CalDAV section lives there as a tab — no longer in the core settings page).
  ModuleSettingsRegistry.register('tasks')

  // Declare the notification activities shown in the core Settings → Notifications matrix.
  NotificationRegistry.register({
    moduleId: 'tasks',
    title: 'Tâches',
    order: 10,
    activities: [
      { id: 'task_assigned', label: 'Une tâche vous est assignée', emailDefault: true, pushDefault: true },
      { id: 'task_due', label: 'Une tâche arrive à échéance', pushDefault: true },
      { id: 'board_changed', label: 'Un tableau, une liste ou une carte est modifié' },
      { id: 'card_comment', label: 'Un commentaire est ajouté sur une carte', pushDefault: true },
    ],
  })

  WidgetRegistry.register({ id: 'tasks-due', moduleId: 'tasks', Component: TasksDueWidget, size: 'medium', order: 12 })

  useSidebarStore.getState().register({
    moduleId:          'tasks',
    routePrefix:       '/tasks',
    newButtonLabelKey: 'tasks:create',
    NewActions:        TasksCreateMenu,
    SidebarBody:       TasksSidebarBody,
    collapsedBody:     true,
  })

  useToolbarStore.getState().register({
    moduleId:         'tasks',
    routePrefix:      '/tasks',
    ToolbarComponent: TasksToolbar,
    noPadding:        true,
  })

  useSearchStore.getState().register({
    moduleId:       'tasks',
    routePrefix:    '/tasks',
    placeholder:    'Search tasks…',
    placeholderKey: 'tasks:search_ph',
    onSearch:       (q) => useTasksStore.getState().setSearchQuery(q),
    FilterPanel:    TasksFilterPanel,
  })

  useRightPanelStore.getState().registerEntry({
    moduleId:       'tasks',
    icon:           CheckSquare,
    label:          'Tasks',
    panelComponent: TasksMiniPanel,
    openPath:       '/tasks',
  })

  const TasksApp          = lazy(() => import('./TasksApp'))
  const TasksSettingsPage = lazy(() => import('./TasksSettingsPage'))

  RouteRegistry.register('tasks',              TasksApp)
  RouteRegistry.register('tasks/boards',       TasksApp)
  RouteRegistry.register('tasks/boards/:id',   TasksApp)
  RouteRegistry.register('tasks/settings',     TasksSettingsPage)

  // Surcharge du calendrier agenda : tasks superpose ses échéances (point
  // d'extension générique — agenda n'a aucune référence à tasks).
  ExtensionRegistry.register(CALENDAR_OVERLAY, 'tasks', {
    fetch: async (fromISO, toISO) => {
      try {
        const [tasks, boards] = await Promise.all([
          tasksApi.listTasks({ due_after: fromISO, due_before: toISO }),
          tasksApi.listBoards(),
        ])
        const boardColor = new Map(boards.map(b => [b.id, b.color]))
        return tasks
          .filter(t => t.due_at)
          .map<CalendarOverlayItem>(t => ({
            id:    `task-${t.id}`,
            date:  format(parseISO(t.due_at as string), 'yyyy-MM-dd'),
            title: t.title,
            color: t.color ?? boardColor.get(t.board_id) ?? '#1a73e8',
            done:  t.status === 'done',
            link:  `/tasks/boards/${t.board_id}`,
          }))
      } catch {
        return []
      }
    },
  } satisfies CalendarOverlayProvider)
}
