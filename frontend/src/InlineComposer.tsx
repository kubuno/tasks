import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { AlignLeft, Clock, Repeat } from 'lucide-react'
import { MenuDropdown, DatePicker, type MenuItem, type MenuDropdownPos } from '@ui'
import { addDays, toISODateTimeLocal } from '@kubuno/sdk'
import { TaskCircle } from './TaskIcons'

export interface DraftTask {
  title: string
  description: string
  /** Local `YYYY-MM-DDTHH:mm`, converted to an instant on submit. */
  dueLocal: string | null
  rrule: string | null
}

interface Props {
  onSubmit: (draft: DraftTask) => void
  onClose: () => void
}

/** 09:00 on the given day — a due date without a stated time means "that morning". */
function morningOf(d: Date): string {
  const at = new Date(d)
  at.setHours(9, 0, 0, 0)
  return toISODateTimeLocal(at)
}

/**
 * The in-place task editor that opens under "Add a task".
 *
 * It stays open after each submit so a burst of tasks can be typed one after
 * the other; it closes on Escape, or when the pointer leaves it with nothing
 * typed. A half-typed title is never thrown away silently: leaving the editor
 * with text in it saves that task.
 */
export default function InlineComposer({ onSubmit, onClose }: Props) {
  const { t } = useTranslation('tasks')
  const [title, setTitle] = useState('')
  const [description, setDescription] = useState('')
  const [dueLocal, setDueLocal] = useState<string | null>(null)
  const [rrule, setRrule] = useState<string | null>(null)
  const [showPicker, setShowPicker] = useState(false)
  const [repeatMenu, setRepeatMenu] = useState<MenuDropdownPos | null>(null)

  const rootRef  = useRef<HTMLDivElement>(null)
  const titleRef = useRef<HTMLInputElement>(null)
  // The submit path reads these, and it runs from a blur listener that closes
  // over the first render — a ref keeps it looking at what is on screen now.
  const draftRef = useRef<DraftTask>({ title: '', description: '', dueLocal: null, rrule: null })
  draftRef.current = { title, description, dueLocal, rrule }

  useEffect(() => { titleRef.current?.focus() }, [])

  const submit = (keepOpen: boolean) => {
    const d = draftRef.current
    if (d.title.trim()) onSubmit({ ...d, title: d.title.trim() })
    if (keepOpen) {
      setTitle(''); setDescription(''); setDueLocal(null); setRrule(null); setShowPicker(false)
      titleRef.current?.focus()
    } else {
      onClose()
    }
  }

  // Clicking anywhere outside commits what was typed and closes the editor.
  useEffect(() => {
    const onPointerDown = (e: PointerEvent) => {
      const el = rootRef.current
      if (!el || el.contains(e.target as Node)) return
      // The date popover and the repeat menu render in a portal, outside this
      // subtree — closing on their clicks would cancel the very edit they make.
      if ((e.target as HTMLElement).closest('[data-kb-portal], [role="dialog"], [role="menu"]')) return
      submit(false)
    }
    document.addEventListener('pointerdown', onPointerDown, true)
    return () => document.removeEventListener('pointerdown', onPointerDown, true)
  }, []) // eslint-disable-line react-hooks/exhaustive-deps

  const REPEATS: { key: string; rule: string | null }[] = [
    { key: 'repeat_never',   rule: null },
    { key: 'repeat_daily',   rule: 'FREQ=DAILY' },
    { key: 'repeat_weekly',  rule: 'FREQ=WEEKLY' },
    { key: 'repeat_monthly', rule: 'FREQ=MONTHLY' },
    { key: 'repeat_yearly',  rule: 'FREQ=YEARLY' },
  ]
  const repeatItems: MenuItem[] = REPEATS.map(r => ({
    type: 'action',
    label: t(r.key),
    checked: rrule === r.rule,
    onClick: () => { setRrule(r.rule); setRepeatMenu(null) },
  }))

  const chip = (label: string, active: boolean, onClick: () => void) => (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={active}
      className={`rounded-full border px-3 py-1 text-xs leading-5 transition-colors
        ${active
          ? 'border-primary bg-primary-light text-primary-hover'
          : 'border-border text-text-secondary hover:bg-surface-2'}`}
    >
      {label}
    </button>
  )

  const todayValue    = morningOf(new Date())
  const tomorrowValue = morningOf(addDays(new Date(), 1))

  return (
    <div ref={rootRef} className="mx-2 rounded-xl bg-surface-1 px-2 py-2">
      <div className="flex items-start gap-3 pl-1">
        <span className="mt-[5px] flex-shrink-0 text-text-secondary"><TaskCircle done={false} /></span>
        <div className="min-w-0 flex-1">
          <input
            ref={titleRef}
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') { e.preventDefault(); submit(true) }
              if (e.key === 'Escape') { e.preventDefault(); onClose() }
            }}
            placeholder={t('task_title_ph')}
            aria-label={t('task_title_ph')}
            className="w-full bg-transparent text-sm leading-5 text-text-nav outline-none
                       placeholder:text-text-tertiary"
          />
          <div className="mt-1.5 flex items-center gap-2">
            <AlignLeft size={16} className="flex-shrink-0 text-text-secondary" aria-hidden="true" />
            <input
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') { e.preventDefault(); submit(true) }
                if (e.key === 'Escape') { e.preventDefault(); onClose() }
              }}
              placeholder={t('task_details_ph')}
              aria-label={t('task_details_ph')}
              className="w-full bg-transparent text-xs leading-5 text-text-nav outline-none
                         placeholder:text-text-tertiary"
            />
          </div>
        </div>
      </div>

      <div className="mt-2 flex items-center gap-2 pl-9">
        {chip(t('chip_today'),    dueLocal === todayValue,    () => { setDueLocal(dueLocal === todayValue ? null : todayValue); setShowPicker(false) })}
        {chip(t('chip_tomorrow'), dueLocal === tomorrowValue, () => { setDueLocal(dueLocal === tomorrowValue ? null : tomorrowValue); setShowPicker(false) })}
        <button
          type="button"
          onClick={() => setShowPicker(v => !v)}
          title={t('pick_datetime')}
          aria-label={t('pick_datetime')}
          aria-expanded={showPicker}
          className={`rounded-full border p-1.5 transition-colors
            ${showPicker || (dueLocal && dueLocal !== todayValue && dueLocal !== tomorrowValue)
              ? 'border-primary bg-primary-light text-primary-hover'
              : 'border-border text-text-secondary hover:bg-surface-2'}`}
        >
          <Clock size={15} />
        </button>

        <button
          type="button"
          onClick={(e) => {
            const r = e.currentTarget.getBoundingClientRect()
            setRepeatMenu({ top: r.bottom + 4, left: r.left - 140 })
          }}
          title={t('repeat')}
          aria-label={t('repeat')}
          className={`ml-auto rounded p-1.5 transition-colors
            ${rrule ? 'text-primary' : 'text-text-secondary hover:bg-surface-2'}`}
        >
          <Repeat size={16} />
        </button>
      </div>

      {showPicker && (
        <div className="mt-2 pl-9 pr-2">
          <DatePicker
            mode="datetime"
            size="sm"
            clearable
            value={dueLocal}
            onChange={(v) => setDueLocal(v)}
            label={t('due_date')}
          />
        </div>
      )}

      {repeatMenu && <MenuDropdown items={repeatItems} pos={repeatMenu} onClose={() => setRepeatMenu(null)} />}
    </div>
  )
}
