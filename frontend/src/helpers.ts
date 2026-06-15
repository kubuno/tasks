import type { TaskStatus } from './api'

/** Priorité iCal 0-9 → niveau UI. */
export function priorityLevel(p: number): 'none' | 'low' | 'medium' | 'high' {
  if (p >= 1 && p <= 4) return 'high'
  if (p === 5) return 'medium'
  if (p >= 6 && p <= 9) return 'low'
  return 'none'
}

export function levelToPriority(level: 'none' | 'low' | 'medium' | 'high'): number {
  switch (level) {
    case 'high':   return 1
    case 'medium': return 5
    case 'low':    return 9
    default:       return 0
  }
}

export const PRIORITY_COLORS: Record<string, string> = {
  high:   '#d93025',
  medium: '#f9ab00',
  low:    '#1a73e8',
  none:   '#bdc1c6',
}

export const STATUS_ORDER: TaskStatus[] = ['open', 'in_progress', 'done', 'cancelled']

export function isOverdue(due: string | null, status: TaskStatus): boolean {
  if (!due || status === 'done' || status === 'cancelled') return false
  return new Date(due).getTime() < Date.now()
}

/** Date courte locale-agnostique (le composant peut surcharger avec date-fns). */
export function shortDate(iso: string | null): string {
  if (!iso) return ''
  const d = new Date(iso)
  return d.toLocaleDateString(undefined, { day: 'numeric', month: 'short' })
}

export function shortDateTime(iso: string | null, allDay: boolean): string {
  if (!iso) return ''
  const d = new Date(iso)
  if (allDay) return d.toLocaleDateString(undefined, { day: 'numeric', month: 'short' })
  return d.toLocaleString(undefined, { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' })
}
