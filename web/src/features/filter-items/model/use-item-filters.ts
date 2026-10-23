import { useCallback, useMemo } from 'react'
import { useSearchParams } from 'react-router'
import type { Category, Kind } from '@/shared/api'
import { searchParamKeys } from '@/shared/config'
import { setIncludeDescendants, useIncludeDescendants } from './descendants'
import type { ItemFilters } from './filter-items'

function list<T extends string>(params: URLSearchParams, key: string): T[] {
  return (params.get(key) ?? '').split(',').filter((value) => value !== '') as T[]
}

export function useItemFilters() {
  const [params, setParams] = useSearchParams()
  const includeDescendants = useIncludeDescendants()

  const filters = useMemo<ItemFilters>(
    () => ({
      query: params.get(searchParamKeys.query) ?? '',
      orders: {},
      node: params.get(searchParamKeys.node),
      kinds: list<Kind>(params, searchParamKeys.kinds),
      categories: list<Category>(params, searchParamKeys.categories),
      includeArchived: params.get(searchParamKeys.archived) === '1',
      includeDescendants,
    }),
    [params, includeDescendants],
  )

  const update = useCallback(
    (key: string, value: string | null) =>
      setParams(
        (current) => {
          const next = new URLSearchParams(current)
          if (value === null || value === '') {
            next.delete(key)
          } else {
            next.set(key, value)
          }
          return next
        },
        { replace: true },
      ),
    [setParams],
  )

  return {
    filters,
    setQuery: (query: string) => update(searchParamKeys.query, query),
    setKinds: (kinds: Kind[]) => update(searchParamKeys.kinds, kinds.join(',')),
    setCategories: (categories: Category[]) => update(searchParamKeys.categories, categories.join(',')),
    setIncludeArchived: (include: boolean) => update(searchParamKeys.archived, include ? '1' : null),
    setIncludeDescendants,
  }
}
