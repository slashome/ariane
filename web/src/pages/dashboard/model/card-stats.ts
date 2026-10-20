import { categoryOrder, isClosed } from '@/entities/item'
import type { Category, Item, NodeSummary } from '@/shared/api'
import { isWithinNode } from '@/shared/lib'

export interface CardStats {
  counts: Record<Category, number>
  total: number
  quickWins: Item[]
  blockers: Item[]
  lastChange: string | null
}

const priorityRank: Record<string, number> = { high: 0, medium: 1, low: 2 }
const smallSizes = ['XS', 'S']

function byPriorityThenId(left: Item, right: Item): number {
  const rank = (item: Item) => (item.priority === null ? 3 : (priorityRank[item.priority] ?? 3))
  return rank(left) - rank(right) || (left.id ?? '').localeCompare(right.id ?? '')
}

export function cardStats(items: Item[], nodes: NodeSummary[], path: string): CardStats {
  const live = items.filter((item) => !item.archived && isWithinNode(item.node, path))
  const counts = Object.fromEntries(categoryOrder.map((category) => [category, 0])) as Record<Category, number>
  for (const item of live) {
    if (item.category !== null) {
      counts[item.category] += 1
    }
  }
  const quickWins = live
    .filter(
      (item) =>
        item.size !== null && smallSizes.includes(item.size) && item.impact === 'high' && !isClosed(item.category),
    )
    .sort(byPriorityThenId)
    .slice(0, 3)
  const blockers = live
    .filter((item) => item.category === 'blocked')
    .sort(byPriorityThenId)
    .slice(0, 3)
  const lastChange =
    nodes
      .filter((node) => isWithinNode(node.path, path))
      .map((node) => node.last_change ?? null)
      .filter((date): date is string => date !== null)
      .sort()
      .at(-1) ?? null
  return {
    counts,
    total: Object.values(counts).reduce((sum, count) => sum + count, 0),
    quickWins,
    blockers,
    lastChange,
  }
}
