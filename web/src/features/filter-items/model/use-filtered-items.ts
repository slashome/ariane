import { useMemo } from 'react'
import { enumValues, useSchema } from '@/entities/schema'
import type { Category, Item, Kind } from '@/shared/api'
import { filterItems } from './filter-items'
import { useItemFilters } from './use-item-filters'

// The items a view shows: the node scope, the archive toggle and the search field of the URL, applied to
// the kinds and categories the view is about, sizes and the like compared along the schema's order.
// `total` counts them before the search field. `anyNode` ignores the node scope, for a list whose items
// were chosen otherwise, such as the tasks of a story.
export function useFilteredItems(items: Item[], view: { kinds?: Kind[]; categories?: Category[]; anyNode?: boolean } = {}) {
  const { filters } = useItemFilters()
  const { data: schema } = useSchema()
  const kinds = view.kinds?.join(',') ?? ''
  const categories = view.categories?.join(',') ?? ''
  const anyNode = view.anyNode ?? false
  return useMemo(() => {
    const scoped = {
      ...filters,
      node: anyNode ? null : filters.node,
      orders: { size: enumValues(schema, 'size'), impact: enumValues(schema, 'impact'), priority: enumValues(schema, 'priority') },
      kinds: kinds === '' ? [] : (kinds.split(',') as Kind[]),
      categories: categories === '' ? [] : (categories.split(',') as Category[]),
    }
    return { items: filterItems(items, scoped), total: filterItems(items, { ...scoped, query: '' }).length }
  }, [items, filters, schema, kinds, categories, anyNode])
}
