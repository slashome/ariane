import { compareNatural, groupBy, sortByManualOrder, type Group } from '@/entities/item'
import type { Item } from '@/shared/api'

export type LaneOrder = 'manual' | 'recent'

export interface Lane {
  id: string
  status: string
  label: string
  hint: string
  items: Item[]
}

const lanes: Omit<Lane, 'id' | 'items'>[] = [
  { status: 'open', label: 'Open', hint: 'Still being thought through' },
  { status: 'trial', label: 'On trial', hint: 'Applied, not confirmed yet' },
  { status: 'settled', label: 'Settled', hint: 'Decided' },
]

// Most recently changed first: the reflexion someone is working on rises to the top of its lane.
const byRecency = (left: Item, right: Item) =>
  compareNatural(right.updated ?? right.created ?? '', left.updated ?? left.created ?? '') ||
  compareNatural(left.reference ?? left.path, right.reference ?? right.path)

export function buildLanes(reflexions: Item[], statuses: string[], order: LaneOrder = 'manual'): Lane[] {
  const sorted = order === 'recent' ? [...reflexions].sort(byRecency) : sortByManualOrder(reflexions, (item) => item)
  return lanes
    .filter((lane) => statuses.length === 0 || statuses.includes(lane.status))
    .map((lane) => ({ ...lane, id: `lane:${lane.status}`, items: sorted.filter((item) => item.status === lane.status) }))
}

const months = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December']

function monthLabel(key: string): string {
  const [year, month] = key.split('-')
  return `${months[Number(month) - 1] ?? month} ${year}`
}

// The settled reflexions by the month of their latest change, newest month first: the date a
// reflexion was settled is not written down, its last commit stands for it.
export function journal(reflexions: Item[]): Group<Item>[] {
  const settled = reflexions.filter((item) => item.status === 'settled').sort(byRecency)
  const groups = groupBy(settled, (item) => (item.updated ?? item.created)?.slice(0, 7) ?? null, 'Undated', monthLabel)
  const dated = groups.filter((group) => group.key !== '').reverse()
  return [...dated, ...groups.filter((group) => group.key === '')]
}
