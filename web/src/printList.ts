import type { Task } from './api'

/**
 * Prints one list on its own.
 *
 * The application shell is a full-height flex layout with its own scroll
 * containers, which browsers print badly (one page, clipped). So rather than
 * trying to coax the live DOM into a printable shape, this builds a plain
 * document fragment holding just the list, hides everything else for the
 * duration of the print, and takes it all back down afterwards.
 */
export function printTaskList(
  title: string,
  open: Task[],
  done: Task[],
  childrenOf: Map<string, Task[]>,
  /** Already-translated heading for the completed block, e.g. "Completed (3)". */
  doneLabel: string,
): void {
  const root = document.createElement('div')
  root.id = 'tasks-print-root'

  const style = document.createElement('style')
  style.id = 'tasks-print-style'
  style.textContent = `
    #tasks-print-root { display: none; }
    @media print {
      body > *:not(#tasks-print-root) { display: none !important; }
      #tasks-print-root {
        display: block !important;
        padding: 24px;
        font: 12pt/1.5 system-ui, sans-serif;
        color: #000;
      }
      #tasks-print-root h1 { font-size: 18pt; font-weight: 600; margin: 0 0 16px; }
      #tasks-print-root h2 { font-size: 12pt; font-weight: 600; margin: 20px 0 8px; }
      #tasks-print-root li { list-style: none; margin: 0 0 6px; page-break-inside: avoid; }
      #tasks-print-root ul { margin: 0; padding: 0; }
      #tasks-print-root .sub { margin-left: 22px; }
      #tasks-print-root .box { display: inline-block; width: 11px; height: 11px;
        border: 1px solid #000; border-radius: 50%; margin-right: 8px; vertical-align: -1px; }
      #tasks-print-root .box.on { background: #000; }
      #tasks-print-root .meta { color: #444; font-size: 10pt; margin-left: 19px; }
      #tasks-print-root .done > .label { text-decoration: line-through; color: #444; }
    }`

  const esc = (s: string) =>
    s.replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c] as string))

  const line = (task: Task, depth: number): string => {
    const kids = childrenOf.get(task.id) ?? []
    const isDone = task.status === 'done'
    const meta = [task.description, task.due_at ? new Date(task.due_at).toLocaleString() : '']
      .filter(Boolean).map(x => esc(String(x))).join(' — ')
    return `<li class="${depth ? 'sub' : ''} ${isDone ? 'done' : ''}">`
      + `<span class="box ${isDone ? 'on' : ''}"></span>`
      + `<span class="label">${esc(task.title)}</span>`
      + (meta ? `<div class="meta">${meta}</div>` : '')
      + '</li>'
      + kids.map(k => line(k, depth + 1)).join('')
  }

  root.innerHTML =
    `<h1>${esc(title)}</h1>`
    + `<ul>${open.map(x => line(x, 0)).join('')}</ul>`
    + (done.length ? `<h2>${esc(doneLabel)}</h2><ul>${done.map(x => line(x, 0)).join('')}</ul>` : '')

  document.head.appendChild(style)
  document.body.appendChild(root)

  const cleanup = () => {
    root.remove()
    style.remove()
    window.removeEventListener('afterprint', cleanup)
  }
  window.addEventListener('afterprint', cleanup)
  window.print()
  // Safari never fires `afterprint` in some versions — a timer guarantees the
  // page is put back the way it was found.
  setTimeout(cleanup, 1000)
}
