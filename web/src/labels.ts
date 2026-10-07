/**
 * The instance's labels, on a task.
 *
 * ## Two label systems, and they are not the same thing
 *
 * A task already carries its BOARD's labels: a short, shared vocabulary that
 * belongs to the board and means something to the people working on it. These
 * are the other kind — the instance's own labels, the ones that also go on a
 * file, a note or an event, owned by the person and browsable across modules
 * from one place. Neither replaces the other, so both have their own line and
 * their own name in the form.
 *
 * Nothing is stored here: the core holds the labels and the links, and this
 * module only says which task is being labelled.
 */
import { useQuery } from '@tanstack/react-query'
import { labelsApi, resourceKeyOf } from '@kubuno/sdk'
import type { ComponentType } from 'react'
import * as UI from '@ui'
import type { Task } from './api'
import { taskEnvelope } from './TasksDataCard'

/** What the core calls a task when it stores a link to one. */
export const LABEL_RESOURCE_TYPE = 'task'

export interface LabelOption { id: string; name: string; color: string }

export interface LabelFieldProps {
  options: LabelOption[]
  value: string[]
  onChange: (ids: string[]) => void
  disabled?: boolean
  placeholder?: string
  emptyHint?: string
  searchPlaceholder?: string
}

/**
 * `LabelField` from the shared library, ahead of its publication.
 *
 * The component is a core primitive and lives in `@ui`; at RUNTIME that
 * specifier resolves to the host's own instance, which already has it. Only the
 * TYPE is missing, because a module typechecks against the published
 * `@kubuno/ui`. Delete this block and import from `@ui` once that package is
 * republished and this module's floor bumped.
 */
export const LabelField =
  (UI as unknown as { LabelField: ComponentType<LabelFieldProps> }).LabelField

/**
 * The identity the core files this task under.
 *
 * Taken from the core's own function rather than restated here: the row menu
 * has been attaching labels under that key since before this field existed, and
 * a second definition would quietly split one task's labels into two piles.
 */
export function taskLabelKey(task: Task): string {
  return resourceKeyOf(taskEnvelope(task))
}

/** Every label this person may put on something. */
export function useLabelOptions() {
  return useQuery({
    queryKey: ['core-labels'],
    queryFn: () => labelsApi.list(),
    staleTime: 60_000,
    // An instance whose labels are unreachable shows an empty picker rather
    // than an error in the middle of a task form.
    retry: false,
  })
}

/** The labels already on this task. */
export function useTaskLabels(task?: Task | null) {
  const key = task ? taskLabelKey(task) : null
  return useQuery({
    queryKey: ['task-labels', key],
    queryFn: () => labelsApi.forResource(LABEL_RESOURCE_TYPE, key as string),
    enabled: Boolean(key),
    retry: false,
  })
}

/**
 * Write the label set of a task.
 *
 * Replaces it wholesale, which is what the picker means: what is on screen IS
 * the set. The envelope travels with it so the core can show the task in its
 * own label browser without having to ask this module anything.
 */
export async function saveTaskLabels(task: Task, labelIds: string[]): Promise<void> {
  const envelope = taskEnvelope(task)
  await labelsApi.setForResource({
    module:        'tasks',
    resource_type: LABEL_RESOURCE_TYPE,
    resource_id:   taskLabelKey(task),
    title:         task.title,
    href:          envelope.href,
    envelope,
    label_ids:     labelIds,
  })
}
