import { useMemo } from 'react'
import { filterItems, ItemFiltersBar, useItemFilters } from '@/features/filter-items'
import { useItems } from '../api/use-items'
import { ItemsTable } from './items-table'

export function TasksPage() {
  const { data: items = [], isPending, error } = useItems()
  const { filters } = useItemFilters()
  const visible = useMemo(() => filterItems(items, filters), [items, filters])

  return (
    <div className="flex flex-col gap-4 p-6">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h1 className="text-xl font-semibold">Tasks</h1>
          <p className="text-sm text-muted-foreground">
            {isPending ? 'Loading…' : `${visible.length} of ${items.length} items`}
          </p>
        </div>
        <ItemFiltersBar />
      </div>
      {error ? (
        <p role="alert" className="rounded-md border border-red-500/40 bg-red-500/10 p-3 text-sm">
          Could not load the items: {error.message}
        </p>
      ) : (
        <ItemsTable items={visible} />
      )}
    </div>
  )
}
