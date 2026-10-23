import { useMemo } from 'react'
import { useItems, useItemsWithBody } from '@/entities/item'
import { useFilteredItems, useItemFilters } from '@/features/filter-items'

// The reflexions in scope, statuses from the item list (which edits update at once), bodies from the
// list that carries them, so the search field reads the bodies too. `total` counts them before the search.
export function useReflexions() {
  const { data: items = [], isPending, error } = useItems()
  const { data: withBody = [] } = useItemsWithBody('reflexion')
  const { filters } = useItemFilters()
  const merged = useMemo(() => {
    const bodies = new Map(withBody.map((item) => [item.path, item.body]))
    return items.map((item) => (item.kind === 'reflexion' ? { ...item, body: bodies.get(item.path) } : item))
  }, [items, withBody])
  const { items: reflexions, total } = useFilteredItems(merged, { kinds: ['reflexion'] })
  return { reflexions, total, isPending, error, scope: filters.node }
}
