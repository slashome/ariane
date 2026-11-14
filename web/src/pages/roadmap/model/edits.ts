import type { Item, Pricing } from '@/shared/api'
import type { GanttRow } from './rows'

export const sizes = ['XS', 'S', 'M', 'L', 'XL'] as const

export type Size = (typeof sizes)[number]

export type Place = 'before' | 'after'

export type ItemOf = (row: GanttRow) => Item | undefined

export type DropCheck = { ok: true; other: GanttRow; place: Place } | { ok: false; reason: string }

export function groupOf(row: GanttRow): string {
  return row.key.slice(0, row.key.lastIndexOf('/'))
}

export function itemKey(item: Pick<Item, 'node' | 'id'>): string {
  return `${item.node}#${item.id}`
}

export function isReorderable(row: GanttRow, item: Item | undefined): item is Item {
  return row.level !== 'release' && row.reference !== null && item !== undefined && !item.archived
}

export function hasGrip(row: GanttRow, item: Item | undefined): item is Item {
  return isReorderable(row, item) && item.category !== 'done'
}

export function checkDrop(dragged: GanttRow, over: GanttRow, place: Place, itemOf: ItemOf): DropCheck {
  if (over.key === dragged.key) {
    return { ok: false, reason: 'Drop it before or after another row' }
  }
  if (groupOf(over) !== groupOf(dragged) || over.level !== dragged.level) {
    return { ok: false, reason: 'Drop it among its siblings: same level, same parent' }
  }
  const item = itemOf(dragged)
  const other = itemOf(over)
  if (!isReorderable(dragged, item) || !isReorderable(over, other)) {
    return { ok: false, reason: 'This row has no item to order' }
  }
  if (item.node !== other.node || item.kind !== other.kind) {
    return { ok: false, reason: 'The order is per node and kind: drop it next to an item of its own node and kind' }
  }
  return { ok: true, other: over, place }
}

export function siblingMove(rows: GanttRow[], row: GanttRow, direction: -1 | 1, itemOf: ItemOf): DropCheck {
  const group = groupOf(row)
  const siblings = rows.filter((entry) => entry.level === row.level && groupOf(entry) === group)
  const neighbour = siblings[siblings.findIndex((entry) => entry.key === row.key) + direction]
  if (neighbour === undefined) {
    return { ok: false, reason: direction < 0 ? 'Already first of its group' : 'Already last of its group' }
  }
  return checkDrop(row, neighbour, direction < 0 ? 'before' : 'after', itemOf)
}

export function stretchRefusal(row: GanttRow, item: Item | undefined): string | null {
  if (row.level === 'release') {
    return 'A release is as long as its work'
  }
  if (row.reference === null || item === undefined || item.archived) {
    return 'No item to resize'
  }
  if (row.work === null || item.estimate?.basis === 'tasks') {
    const parts = row.kind === 'epic' ? 'stories' : 'tasks'
    return `Its size is the sum of its ${parts}: resize those instead`
  }
  switch (row.work.stage) {
    case 'done':
      return 'Done work keeps its size'
    case 'testing':
      return 'Its work is done: only its testing remains'
    case 'waiting':
      return 'Its work is done: it waits for validation'
    default:
      return row.work.hours > 0 ? null : 'Nothing left to resize'
  }
}

export function sizeHours(size: Size, pricing: Pricing): number {
  return pricing[size]
}

export function snapSize(hours: number, pricing: Pricing): Size {
  let best: Size = sizes[0]
  for (const size of sizes) {
    if (Math.abs(pricing[size] - hours) < Math.abs(pricing[best] - hours)) {
      best = size
    }
  }
  return best
}

export function stretchedHours(hours: number, width: number, delta: number): number {
  return width <= 0 ? hours : Math.max(0, (hours * (width + delta)) / width)
}

export function currentSize(item: Pick<Item, 'size'>, pricing: Pricing): Size | null {
  const size = item.size ?? pricing.default
  return (sizes as readonly string[]).includes(size) ? (size as Size) : null
}

export function stepSize(item: Pick<Item, 'size'>, pricing: Pricing, step: -1 | 1): Size | null {
  const current = currentSize(item, pricing)
  if (current === null) {
    return null
  }
  return sizes[sizes.indexOf(current) + step] ?? null
}
