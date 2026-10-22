import type { Category, Item } from '@/shared/api'
import { isClosed } from './categories'

export interface Progress {
  counts: Record<Category, number>
  // Items that count toward completion: every categorized item but the dropped ones, as Linear leaves
  // canceled issues out of a milestone's scope.
  total: number
  done: number
  ratio: number
}

const emptyCounts = (): Record<Category, number> => ({
  backlog: 0,
  todo: 0,
  in_progress: 0,
  blocked: 0,
  done: 0,
  dropped: 0,
})

export function progressOf(items: Item[]): Progress {
  const counts = emptyCounts()
  for (const item of items) {
    if (item.category !== null) {
      counts[item.category] += 1
    }
  }
  const total = Object.values(counts).reduce((sum, count) => sum + count, 0) - counts.dropped
  return { counts, total, done: counts.done, ratio: total === 0 ? 0 : counts.done / total }
}

export function indexByReference(items: Item[]): Map<string, Item> {
  return new Map(items.filter((item) => item.reference !== null).map((item) => [item.reference as string, item]))
}

export function childrenOf(items: Item[], reference: string | null): Item[] {
  return reference === null ? [] : items.filter((item) => item.parent === reference)
}

export interface Blockers {
  // Dependencies not done nor dropped yet.
  open: Item[]
  // References that point to no known item.
  missing: string[]
}

export function blockersOf(item: Item, byReference: Map<string, Item>): Blockers {
  const open: Item[] = []
  const missing: string[] = []
  for (const reference of item.depends_on) {
    const target = byReference.get(reference)
    if (target === undefined) {
      missing.push(reference)
    } else if (!isClosed(target.category)) {
      open.push(target)
    }
  }
  return { open, missing }
}

// Orders items so that each one comes after the items it depends on, keeping the given order among
// independent ones; a dependency cycle is broken where it is met.
export function dependencyOrder(items: Item[]): Item[] {
  const byReference = indexByReference(items)
  const placed = new Set<string>()
  const visiting = new Set<string>()
  const ordered: Item[] = []
  const visit = (item: Item) => {
    if (placed.has(item.path) || visiting.has(item.path)) {
      return
    }
    visiting.add(item.path)
    for (const reference of item.depends_on) {
      const target = byReference.get(reference)
      if (target !== undefined) {
        visit(target)
      }
    }
    visiting.delete(item.path)
    placed.add(item.path)
    ordered.push(item)
  }
  items.forEach(visit)
  return ordered
}
