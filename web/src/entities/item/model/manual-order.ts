import type { Item } from '@/shared/api'
import { compareNatural } from './groups'

const levels = ['low', 'medium', 'high']

function weight(priority: string | null): number {
  return priority === null ? -1 : levels.indexOf(priority)
}

function rankOf(item: Item): number | null {
  return item.archived ? null : item.rank
}

export function compareManualOrder(left: Item, right: Item): number {
  const leftRank = rankOf(left)
  const rightRank = rankOf(right)
  const byRank =
    leftRank !== null && rightRank !== null
      ? leftRank - rightRank
      : leftRank !== null
        ? -1
        : rightRank !== null
          ? 1
          : weight(right.priority) - weight(left.priority)
  return byRank !== 0 ? byRank : compareNatural(left.id ?? '\uffff', right.id ?? '\uffff')
}
