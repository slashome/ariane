import type { Item, Place } from '@/shared/api'
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

export function sortByManualOrder<T>(entries: T[], itemOf: (entry: T) => Item): T[] {
  const nodes = new Map<string, number>()
  const kinds = new Map<string, number>()
  for (const entry of entries) {
    const item = itemOf(entry)
    if (!nodes.has(item.node)) {
      nodes.set(item.node, nodes.size)
    }
    const kind = `${item.node}#${item.kind}`
    if (!kinds.has(kind)) {
      kinds.set(kind, kinds.size)
    }
  }
  const nodeOf = (item: Item) => nodes.get(item.node) ?? 0
  const kindOf = (item: Item) => kinds.get(`${item.node}#${item.kind}`) ?? 0
  return [...entries].sort((left, right) => {
    const [a, b] = [itemOf(left), itemOf(right)]
    return nodeOf(a) - nodeOf(b) || kindOf(a) - kindOf(b) || compareManualOrder(a, b)
  })
}

export function placeInOrder(items: Item[], moved: Item, place: Place, other: Item | null, changed: Partial<Item> = {}): Item[] {
  const updated = { ...moved, ...changed }
  const rest = items.filter((item) => item.path !== moved.path)
  const peers = rest.filter((item) => item.node === moved.node && item.kind === moved.kind && !item.archived)
  const anchor = place === 'top' ? peers[0] : place === 'bottom' ? peers.filter((item) => item.rank !== null).at(-1) : (other ?? undefined)
  if (anchor === undefined || place === 'clear') {
    return items.map((item) => (item.path === moved.path ? updated : item))
  }
  const at = rest.findIndex((item) => item.path === anchor.path)
  const index = place === 'top' || place === 'before' ? at : at + 1
  return [...rest.slice(0, index), updated, ...rest.slice(index)]
}
