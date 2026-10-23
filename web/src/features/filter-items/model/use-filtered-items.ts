import { useMemo } from 'react'
import { enumValues, useSchema } from '@/entities/schema'
import type { Category, Item, Kind } from '@/shared/api'
import { filterItems } from './filter-items'
import { useItemFilters } from './use-item-filters'

// The items a view shows: the node scope, the archive toggle and the search field of the URL, applied to
// the kinds and categories the view is about, sizes and the like compared along the schema's order.
// `total` counts them before the search field.
export function useFilteredItems(items: Item[], view: { kinds?: Kind[]; categories?: Category[] } = {}) {
  const { filters } = useItemFilters()
  const { data: schema } = useSchema()
  const kinds = view.kinds?.join(',') ?? ''
  const categories = view.categories?.join(',') ?? ''
  return useMemo(() => {
    const scoped = {
      ...filters,
      orders: { size: enumValues(schema, 'size'), impact: enumValues(schema, 'impact'), priority: enumValues(schema, 'priority') },
      kinds: kinds === '' ? [] : (kinds.split(',') as Kind[]),
      categories: categories === '' ? [] : (categories.split(',') as Category[]),
    }
    return { items: filterItems(items, scoped), total: filterItems(items, { ...scoped, query: '' }).length }
  }, [items, filters, schema, kinds, categories])
}
