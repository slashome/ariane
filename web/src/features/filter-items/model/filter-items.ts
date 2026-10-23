import type { Category, Item, Kind } from '@/shared/api'
import { isWithinNode } from '@/shared/lib'
import { matchesQuery } from './match'
import { parseQuery } from './query'
import type { RankOrders } from './vocabulary'

export interface ItemFilters {
  // The search field as typed: filters such as `status:` or `size:<=M`, and free words.
  query: string
  // The schema's order of sizes, impacts and priorities, which `size:<=M` compares along.
  orders: RankOrders
  node: string | null
  kinds: Kind[]
  categories: Category[]
  includeArchived: boolean
  // With a node in scope, keep the items of its sub-nodes too; only its own items otherwise.
  includeDescendants: boolean
}

export const emptyFilters: ItemFilters = {
  query: '',
  orders: {},
  node: null,
  kinds: [],
  categories: [],
  includeArchived: false,
  includeDescendants: false,
}

export type Scope = Pick<ItemFilters, 'node' | 'includeArchived' | 'includeDescendants'>

// Whether a node path is in scope: every node without a scope, the node alone, or with its sub-nodes.
export function inScope(path: string, node: string | null, includeDescendants: boolean): boolean {
  return node === null || (includeDescendants ? isWithinNode(path, node) : path === node)
}

// The items every view shows: those of the node in scope, archived ones only when the toggle says so.
export function scopeItems(items: Item[], scope: Scope, kinds: Kind[] = []): Item[] {
  return filterItems(items, {
    ...emptyFilters,
    node: scope.node,
    includeArchived: scope.includeArchived,
    includeDescendants: scope.includeDescendants,
    kinds,
  })
}

export function filterItems(items: Item[], filters: ItemFilters): Item[] {
  const query = parseQuery(filters.query)
  return items.filter(
    (item) =>
      (filters.includeArchived || !item.archived) &&
      inScope(item.node, filters.node, filters.includeDescendants) &&
      (filters.kinds.length === 0 || filters.kinds.includes(item.kind)) &&
      (filters.categories.length === 0 || (item.category !== null && filters.categories.includes(item.category))) &&
      matchesQuery(item, query, filters.orders),
  )
}
