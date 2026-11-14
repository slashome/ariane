import type { Release, ReleaseChange } from '@/shared/api'

export type ColumnAction = 'rename' | 'left' | 'right' | 'parking' | 'close' | 'remove'

export interface ColumnMenuEntry {
  action: ColumnAction
  label: string
  disabled: boolean
  reason: string | null
}

function entry(action: ColumnAction, label: string, reason: string | null): ColumnMenuEntry {
  return { action, label, disabled: reason !== null, reason }
}

function shown(references: string[]): string {
  const listed = references.slice(0, 3).join(', ')
  return references.length > 3 ? `${listed} and ${references.length - 3} more` : listed
}

export function moveTarget(shownOrder: Release[], name: string, direction: 'left' | 'right'): Release | null {
  const index = shownOrder.findIndex((release) => release.name === name)
  const neighbour = shownOrder[direction === 'left' ? index - 1 : index + 1]
  return index === -1 || neighbour === undefined ? null : neighbour
}

function moveReason(shownOrder: Release[], release: Release, direction: 'left' | 'right'): string | null {
  const neighbour = moveTarget(shownOrder, release.name, direction)
  if (neighbour === null) {
    return direction === 'left' ? 'Already first' : 'Already last'
  }
  return neighbour.parking !== release.parking ? 'Parking releases stay after the scheduled ones' : null
}

function closeReason(release: Release): string | null {
  return release.parking ? 'A parking release is never closed' : null
}

export function columnMenu(shownOrder: Release[], release: Release, usedBy: string[], inheritedFrom: string | null): ColumnMenuEntry[] {
  const locked = inheritedFrom === null ? null : `The list is declared in ${inheritedFrom === '' ? 'the home node' : inheritedFrom}`
  const closed = release.closed ? `Closed on ${release.closed}` : null
  return [
    entry('rename', 'Rename…', locked),
    entry('left', 'Move left', locked ?? closed ?? moveReason(shownOrder, release, 'left')),
    entry('right', 'Move right', locked ?? closed ?? moveReason(shownOrder, release, 'right')),
    entry('parking', release.parking ? 'Schedule it (not parking)' : 'Make it a parking release', locked ?? closed),
    entry('close', 'Close release…', locked ?? closed ?? closeReason(release)),
    entry('remove', 'Remove', locked ?? (usedBy.length > 0 ? `Still the release of ${shown(usedBy)}` : null)),
  ]
}

export function moveRequest(node: string, shownOrder: Release[], name: string, direction: 'left' | 'right'): ReleaseChange | null {
  const neighbour = moveTarget(shownOrder, name, direction)
  if (neighbour === null) {
    return null
  }
  return { node, name, place: direction === 'left' ? 'before' : 'after', other: neighbour.name }
}

export interface PlaceChoice {
  value: string
  label: string
  change: Pick<ReleaseChange, 'place' | 'other'>
}

export const lastPlace = 'last'

export function placeChoices(shownOrder: Release[]): PlaceChoice[] {
  const firstParking = shownOrder.find((release) => release.parking)
  const last: PlaceChoice = {
    value: lastPlace,
    label: firstParking ? `Last, before ${firstParking.name}` : 'Last',
    change: firstParking ? { place: 'before', other: firstParking.name } : { place: 'bottom' },
  }
  const scheduled = shownOrder.filter((release) => !release.parking)
  if (scheduled.length === 0) {
    return [last]
  }
  return [
    last,
    { value: 'top', label: 'First', change: { place: 'top' } },
    ...scheduled.slice(0, -1).map((release) => ({
      value: `after:${release.name}`,
      label: `After ${release.name}`,
      change: { place: 'after' as const, other: release.name },
    })),
  ]
}
