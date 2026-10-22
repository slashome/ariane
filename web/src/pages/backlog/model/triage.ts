import { compareNatural } from '@/entities/item'
import type { Item } from '@/shared/api'

export type TriageField = 'priority' | 'impact' | 'size'

export type BacklogSort = 'triage' | TriageField | 'created'

export const sorts: { value: BacklogSort; label: string }[] = [
  { value: 'triage', label: 'Priority, impact, size' },
  { value: 'priority', label: 'Priority' },
  { value: 'impact', label: 'Impact' },
  { value: 'size', label: 'Size (smallest first)' },
  { value: 'created', label: 'Oldest first' },
]

// Higher is more urgent or more valuable; an unset value ranks below every set one.
function level(values: string[], value: string | null): number {
  return value === null ? -1 : values.indexOf(value)
}

// What a triage pass still has to decide for an item.
export function missingFields(item: Item): TriageField[] {
  return (['priority', 'impact', 'size'] as const).filter((field) => item[field] === null)
}

export function sortBacklog(items: Item[], sort: BacklogSort, levels: string[], sizes: string[]): Item[] {
  const priority = (item: Item) => level(levels, item.priority)
  const impact = (item: Item) => level(levels, item.impact)
  // Small first, unsized last.
  const size = (item: Item) => (item.size === null ? sizes.length : sizes.indexOf(item.size))
  const byId = (left: Item, right: Item) => compareNatural(left.reference ?? left.path, right.reference ?? right.path)
  const comparators: Record<BacklogSort, (left: Item, right: Item) => number> = {
    triage: (left, right) => priority(right) - priority(left) || impact(right) - impact(left) || size(left) - size(right),
    priority: (left, right) => priority(right) - priority(left),
    impact: (left, right) => impact(right) - impact(left),
    size: (left, right) => size(left) - size(right),
    created: (left, right) => compareNatural(left.created ?? '9999', right.created ?? '9999'),
  }
  return [...items].sort((left, right) => comparators[sort](left, right) || byId(left, right))
}

// The next value of a field when its key is pressed: through the vocabulary, then back to unset.
export function cycle(values: string[], current: string | null): string | null {
  if (current === null) {
    return values[0] ?? null
  }
  const index = values.indexOf(current)
  return index < 0 || index === values.length - 1 ? null : (values[index + 1] ?? null)
}
