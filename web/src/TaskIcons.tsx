/**
 * The few glyphs this module draws itself.
 *
 * They are inline SVG rather than icon-font entries because each one carries a
 * meaning no generic icon set expresses: a circle that is both a checkbox and a
 * hit target, and a "add a task" mark that is that same circle with a plus.
 * Keeping them here means one stroke width and one geometry across the module.
 */

/** The task checkbox: an outlined circle, filled with a tick once done. */
export function TaskCircle({ done, size = 20 }: { done: boolean; size?: number }) {
  return done ? (
    <svg width={size} height={size} viewBox="0 0 20 20" fill="none" aria-hidden="true">
      <circle cx="10" cy="10" r="9" fill="currentColor" />
      <path d="M6 10.2l2.6 2.6L14 7.4" stroke="#fff" strokeWidth="1.7"
            strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  ) : (
    <svg width={size} height={size} viewBox="0 0 20 20" fill="none" aria-hidden="true">
      <circle cx="10" cy="10" r="8.25" stroke="currentColor" strokeWidth="1.5" />
    </svg>
  )
}

/** "Add a task": the same circle, ticked, with a plus tucked at its corner. */
export function AddTaskIcon({ size = 20 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 20 20" fill="none" aria-hidden="true">
      {/* Ticked circle, kept clear of the bottom-right corner so the plus stands alone. */}
      <circle cx="8" cy="8" r="6.6" stroke="currentColor" strokeWidth="1.6" />
      <path d="M5 8.2l2.2 2.2 4-4.8" stroke="currentColor" strokeWidth="1.6"
            strokeLinecap="round" strokeLinejoin="round" />
      {/* A bold, detached plus badge — the whole point of this icon must read at a glance. */}
      <path d="M16 13.3v5.4M13.3 16h5.4" stroke="currentColor" strokeWidth="2.4"
            strokeLinecap="round" />
    </svg>
  )
}

/** The grab bar shown at the top of a list column while the pointer is over it. */
export function DragBar() {
  return <span className="block h-1 w-9 rounded-full bg-border-strong/70" />
}

/**
 * The drawing shown where a list has no task yet.
 *
 * A clipboard checklist with two lines ticked off: it reads as "tasks" at a
 * glance, and the green ticks carry the "nothing left / all done" message that
 * fits both an empty list and a fully completed one.
 */
export function EmptyTasksArt({ className = '' }: { className?: string }) {
  return (
    <svg viewBox="0 0 132 96" className={className} role="presentation" aria-hidden="true">
      {/* clip at the top of the board */}
      <rect x="54" y="12" width="24" height="12" rx="4" fill="#a8c7fa" />
      {/* clipboard */}
      <rect x="34" y="18" width="64" height="62" rx="9" fill="#ffffff" stroke="#dadce0" strokeWidth="2" />
      {/* row 1 — ticked */}
      <circle cx="50" cy="38" r="6" fill="#1e8e3e" />
      <path d="M47.2 38.2l1.9 1.9 3.7-4.1" stroke="#fff" strokeWidth="1.8"
            strokeLinecap="round" strokeLinejoin="round" fill="none" />
      <rect x="60" y="35" width="28" height="6" rx="3" fill="#cbe6d3" />
      {/* row 2 — ticked */}
      <circle cx="50" cy="55" r="6" fill="#1e8e3e" />
      <path d="M47.2 55.2l1.9 1.9 3.7-4.1" stroke="#fff" strokeWidth="1.8"
            strokeLinecap="round" strokeLinejoin="round" fill="none" />
      <rect x="60" y="52" width="22" height="6" rx="3" fill="#e8eaed" />
      {/* row 3 — still open */}
      <circle cx="50" cy="72" r="6" fill="none" stroke="#dadce0" strokeWidth="2" />
      <rect x="60" y="69" width="18" height="6" rx="3" fill="#e8eaed" />
    </svg>
  )
}
