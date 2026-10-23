import { isClosed } from '@/entities/item'
import type { Item } from '@/shared/api'
import { foldText } from '@/shared/lib'
import type { Clause, Comparator, ItemQuery } from './query'
import type { RankOrders } from './vocabulary'

function same(left: string | null, right: string): boolean {
  return left !== null && foldText(left) === foldText(right)
}

function haystack(item: Item): string {
  return [item.title, item.id, item.reference, item.node, item.path, item.body]
    .filter((field): field is string => typeof field === 'string')
    .map(foldText)
    .join('\n')
}

function compare(index: number, target: number, comparator: Comparator): boolean {
  switch (comparator) {
    case '=':
      return index === target
    case '<':
      return index < target
    case '<=':
      return index <= target
    case '>':
      return index > target
    case '>=':
      return index >= target
  }
}

function rankOf(order: string[], value: string | null): number {
  return value === null ? -1 : order.findIndex((entry) => same(entry, value))
}

// An item is closed once done, dropped or archived; open otherwise.
function closed(item: Item): boolean {
  return item.archived || (item.category !== null && isClosed(item.category))
}

// A clause still being typed — `status:`, `@`, `""` — filters nothing, negated or not.
function incomplete(clause: Clause): boolean {
  return (
    (clause.type === 'filter' && clause.values.length === 0) ||
    (clause.type === 'person' && clause.handle === '') ||
    (clause.type === 'text' && clause.text === '')
  )
}

function holds(clause: Clause, item: Item, orders: RankOrders, text: () => string): boolean {
  switch (clause.type) {
    case 'text':
    case 'unknown':
      return text().includes(foldText(clause.text))
    case 'person':
      return item.waiting_for.some((handle) => same(handle, clause.handle))
    case 'filter': {
      switch (clause.key) {
        case 'status':
          return clause.values.some((value) => same(item.status, value))
        case 'category':
          return clause.values.some((value) => same(item.category, value))
        case 'is':
          return clause.values.some((value) => (same('closed', value) ? closed(item) : same('open', value) && !closed(item)))
        case 'size':
        case 'impact':
        case 'priority': {
          const order = orders[clause.key] ?? []
          const index = rankOf(order, item[clause.key])
          return (
            index !== -1 &&
            clause.values.some((value) => {
              const target = rankOf(order, value)
              return target !== -1 && compare(index, target, clause.comparator)
            })
          )
        }
      }
    }
  }
}

// Whether an item answers every clause of a query, a negated clause by failing it. Free words are looked
// for in the title, id, reference, node, path and loaded body, case and accents ignored.
export function matchesQuery(item: Item, query: ItemQuery, orders: RankOrders): boolean {
  let folded: string | null = null
  const text = () => (folded ??= haystack(item))
  return query.clauses.every((clause) => incomplete(clause) || holds(clause, item, orders, text) !== clause.negated)
}
