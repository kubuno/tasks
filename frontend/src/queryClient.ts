/**
 * Module-scope handle on the HOST's react-query client.
 *
 * The `shell.new-actions` items run outside any React component, so they
 * cannot call `useQueryClient()`. The client is captured once by
 * `TaskCreateDialog` (slot `app-dialogs`, mounted globally by the shell on
 * every route) and read back here to invalidate queries after a create.
 */
import type { QueryClient } from '@tanstack/react-query'

let client: QueryClient | null = null

export function setSharedQueryClient(qc: QueryClient): void {
  client = qc
}

/** Invalidates each given query key on the host client (no-op before capture). */
export function invalidateQueries(...keys: readonly unknown[][]): void {
  for (const queryKey of keys) client?.invalidateQueries({ queryKey })
}
