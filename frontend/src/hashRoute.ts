// Addressable sidebar views that have no route of their own (task collections
// such as Today/Overdue/Important…) are encoded in the URL hash:
// `/tasks/#<kind>/<id>`.
//
// Keeping the format in a single place lets the sidebar build REAL links
// (`<a href="/tasks/#collection/today">`) while the sidebar reads the state back
// from `useLocation().hash`, so a direct link and the browser Back button both
// select the right collection.

const MODULE_PATH = '/tasks'

/** Build the link target for an addressable sidebar view. */
export function hashTo(kind: string, id: string): string {
  return `${MODULE_PATH}/#${encodeURIComponent(kind)}/${encodeURIComponent(id)}`
}

/** Parse a `location.hash` back into its `{ kind, id }` pair (null if it is not one). */
export function fromHash(hash: string): { kind: string; id: string } | null {
  const m = /^#([^/]+)\/([^/]+)$/.exec(hash)
  if (!m) return null
  return { kind: decodeURIComponent(m[1]), id: decodeURIComponent(m[2]) }
}
