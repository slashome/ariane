import { useCallback, useState } from 'react'
import type { Kind, Roadmap, Unscheduled, UnscheduledReason } from '@/shared/api'

export const unplannedKey = 'unplanned'

export interface ColumnEntry {
  reference: string
  id: string
  kind: Kind
  title: string
  reason: UnscheduledReason
  release: string | null
}

export interface UndatedColumn {
  key: string
  name: string
  parking: boolean
  entries: ColumnEntry[]
}

export function reasonLabel(entry: Pick<ColumnEntry, 'reason' | 'release'>): string {
  switch (entry.reason) {
    case 'parking':
      return `parked in ${entry.release ?? 'a parking release'}`
    case 'closed_release':
      return entry.release === null ? 'left in a closed release' : `left in ${entry.release}, closed`
    case 'no_release':
      return 'no release'
    case 'undeclared_release':
      return entry.release === null ? 'release not declared' : `release ${entry.release} not declared`
    case 'cycle':
      return 'dependency cycle'
    case 'blocked_by_cycle':
      return 'waits on a dependency cycle'
  }
}

function entryOf(unscheduled: Unscheduled): ColumnEntry {
  const at = unscheduled.reference.lastIndexOf('#')
  return {
    reference: unscheduled.reference,
    id: at === -1 ? unscheduled.reference : unscheduled.reference.slice(at + 1),
    kind: unscheduled.kind,
    title: unscheduled.title,
    reason: unscheduled.reason,
    release: unscheduled.release,
  }
}

export function buildColumns(roadmap: Pick<Roadmap, 'unscheduled'>): UndatedColumn[] {
  const parking = new Map<string, UndatedColumn>()
  const unplanned: UndatedColumn = { key: unplannedKey, name: 'Unplanned', parking: false, entries: [] }
  for (const unscheduled of roadmap.unscheduled) {
    if (unscheduled.reason === 'parking') {
      const name = unscheduled.release ?? 'parking'
      const key = `parking:${name}`
      const column = parking.get(key) ?? { key, name, parking: true, entries: [] }
      column.entries.push(entryOf(unscheduled))
      parking.set(key, column)
    } else {
      unplanned.entries.push(entryOf(unscheduled))
    }
  }
  const columns = [...parking.values()]
  return unplanned.entries.length > 0 ? [...columns, unplanned] : columns
}

export function narrowColumns(columns: UndatedColumn[], keep: ReadonlySet<string> | null): UndatedColumn[] {
  if (keep === null) {
    return columns
  }
  return columns
    .map((column) => ({ ...column, entries: column.entries.filter((entry) => keep.has(entry.reference)) }))
    .filter((column) => column.entries.length > 0)
}

export function toggleColumn(unfolded: readonly string[], key: string): string[] {
  return unfolded.includes(key) ? unfolded.filter((entry) => entry !== key) : [...unfolded, key]
}

export function parseUnfolded(stored: unknown): string[] {
  return Array.isArray(stored) ? stored.filter((entry): entry is string => typeof entry === 'string') : []
}

const storageKey = 'ariane.roadmap.unfolded'

function readUnfolded(): string[] {
  try {
    return parseUnfolded(JSON.parse(localStorage.getItem(storageKey) ?? 'null'))
  } catch {
    return []
  }
}

function storeUnfolded(unfolded: string[]) {
  try {
    localStorage.setItem(storageKey, JSON.stringify(unfolded))
  } catch {
    return
  }
}

export function useUnfoldedColumns(): [string[], (key: string) => void] {
  const [unfolded, setUnfolded] = useState(readUnfolded)
  const toggle = useCallback(
    (key: string) =>
      setUnfolded((current) => {
        const next = toggleColumn(current, key)
        storeUnfolded(next)
        return next
      }),
    [],
  )
  return [unfolded, toggle]
}
