import { useMemo } from 'react'
import { useItems, useItemsWithBody } from '@/entities/item'
import { emptyFilters, filterItems, useItemFilters } from '@/features/filter-items'

// The reflexions in scope, statuses from the item list (which edits update at once), bodies from the
// list that carries them.
export function useReflexions() {
  const { data: items = [], isPending, error } = useItems()
  const { data: withBody = [] } = useItemsWithBody('reflexion')
  const { filters } = useItemFilters()
  const reflexions = useMemo(() => {
    const bodies = new Map(withBody.map((item) => [item.path, item.body]))
    return filterItems(items, {
      ...emptyFilters,
      node: filters.node,
      includeArchived: filters.includeArchived,
      includeDescendants: filters.includeDescendants,
      kinds: ['reflexion'],
      query: filters.query,
    }).map((item) => ({ ...item, body: bodies.get(item.path) }))
  }, [items, withBody, filters])
  return { reflexions, isPending, error, scope: filters.node }
}
