import type { Category, Item, Kind } from '@/shared/api'
import { isWithinNode } from '@/shared/lib'

export interface ItemFilters {
  query: string
  node: string | null
  kinds: Kind[]
  categories: Category[]
  includeArchived: boolean
}

export const emptyFilters: ItemFilters = {
  query: '',
  node: null,
  kinds: [],
  categories: [],
  includeArchived: false,
}

function matchesQuery(item: Item, query: string): boolean {
  if (query === '') {
    return true
  }
  const needle = query.toLowerCase()
  return [item.title, item.id, item.node, item.path, item.body].some((field) => field?.toLowerCase().includes(needle))
}

// The items every view shows: those of the node in scope, archived ones only when the toggle says so.
export function scopeItems(items: Item[], scope: { node: string | null; includeArchived: boolean }, kinds: Kind[] = []): Item[] {
  return filterItems(items, { ...emptyFilters, node: scope.node, includeArchived: scope.includeArchived, kinds })
}

export function filterItems(items: Item[], filters: ItemFilters): Item[] {
  const query = filters.query.trim()
  return items.filter(
    (item) =>
      (filters.includeArchived || !item.archived) &&
      (filters.node === null || isWithinNode(item.node, filters.node)) &&
      (filters.kinds.length === 0 || filters.kinds.includes(item.kind)) &&
      (filters.categories.length === 0 || (item.category !== null && filters.categories.includes(item.category))) &&
      matchesQuery(item, query),
  )
}
