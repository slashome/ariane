import { compareNatural } from '@/entities/item'
import type { Item } from '@/shared/api'

export type TriageField = 'priority' | 'impact' | 'size'

export const triageFields: TriageField[] = ['priority', 'impact', 'size']

export function level(values: string[], value: string | null): number {
  return value === null ? -1 : values.indexOf(value)
}

export function missingFields(item: Item): TriageField[] {
  return triageFields.filter((field) => item[field] === null)
}

export function triageOrder(items: Item[], levels: string[], sizes: string[]): Item[] {
  const priority = (item: Item) => level(levels, item.priority)
  const impact = (item: Item) => level(levels, item.impact)
  const size = (item: Item) => (item.size === null ? sizes.length : sizes.indexOf(item.size))
  const byId = (left: Item, right: Item) => compareNatural(left.reference ?? left.path, right.reference ?? right.path)
  return [...items].sort(
    (left, right) => priority(right) - priority(left) || impact(right) - impact(left) || size(left) - size(right) || byId(left, right),
  )
}

export function cycle(values: string[], current: string | null): string | null {
  if (current === null) {
    return values[0] ?? null
  }
  const index = values.indexOf(current)
  return index < 0 || index === values.length - 1 ? null : (values[index + 1] ?? null)
}
