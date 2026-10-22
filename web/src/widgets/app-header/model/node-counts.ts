import { scopeItems } from '@/features/filter-items'
import type { Item } from '@/shared/api'

// How many items each node would show if it were chosen, under the current archive and sub-nodes
// toggles: the number beside each option of the node select.
export function nodeCounts(
  items: Item[],
  paths: string[],
  toggles: { includeArchived: boolean; includeDescendants: boolean },
): Map<string, number> {
  return new Map(paths.map((path) => [path, scopeItems(items, { node: path, ...toggles }).length]))
}
