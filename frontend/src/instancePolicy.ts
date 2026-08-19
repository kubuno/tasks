// What the instance allows, as the module's screens need to know it.
//
// The core's `/modules/tasks/config` deliberately hides instance-scoped settings
// from accounts without the settings privilege, so it cannot answer "may I share
// this board?" for an ordinary user. The module answers that itself, on
// `/tasks/instance-policy`, with the decisions its own screens act on — the same
// ones the server enforces, so the interface never offers an action the backend
// will refuse.
import { useMemo } from 'react'
import { useQuery } from '@tanstack/react-query'
import { api } from '@kubuno/sdk'

export interface InstancePolicy {
  /** May a board be shared with other accounts? */
  allowBoardSharing: boolean
  /** Ceiling on boards owned by one account; 0 = no ceiling. */
  maxBoardsPerUser: number
  /** Ceiling on tasks held by one board; 0 = no ceiling. */
  maxTasksPerBoard: number
  /** Ceiling on the declared size of an attachment, in megabytes; 0 = none. */
  attachmentMaxMb: number
}

/** Permissive until the query answers: the first paint must not flash a screen
 *  that looks locked down, and the server refuses anything it disallows anyway. */
export const INSTANCE_POLICY_DEFAULTS: InstancePolicy = {
  allowBoardSharing: true,
  maxBoardsPerUser:  0,
  maxTasksPerBoard:  0,
  attachmentMaxMb:   0,
}

interface RawPolicy {
  allow_board_sharing?: boolean
  max_boards_per_user?: number
  max_tasks_per_board?: number
  attachment_max_mb?:   number
}

export function useInstancePolicy(): InstancePolicy {
  const { data } = useQuery({
    queryKey: ['tasks-instance-policy'],
    queryFn:  () => api.get<RawPolicy>('/tasks/instance-policy').then(r => r.data),
    staleTime: 5 * 60_000,
  })

  // Memoised: consumers put the result in dependency arrays, and a fresh object
  // on every render would defeat every one of them.
  return useMemo(() => {
    const d = INSTANCE_POLICY_DEFAULTS
    if (!data) return d
    const num = (v: unknown, fallback: number) => (typeof v === 'number' ? v : fallback)
    return {
      allowBoardSharing: data.allow_board_sharing ?? d.allowBoardSharing,
      maxBoardsPerUser:  num(data.max_boards_per_user, d.maxBoardsPerUser),
      maxTasksPerBoard:  num(data.max_tasks_per_board, d.maxTasksPerBoard),
      attachmentMaxMb:   num(data.attachment_max_mb,   d.attachmentMaxMb),
    }
  }, [data])
}
