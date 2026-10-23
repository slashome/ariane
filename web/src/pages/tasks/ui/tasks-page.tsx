import { useItems } from '@/entities/item'
import { ItemFiltersBar, QueryField, useFilteredItems, useItemFilters } from '@/features/filter-items'
import { ItemsTable } from './items-table'

export function TasksPage() {
  const { data: items = [], isPending, error } = useItems()
  const { filters } = useItemFilters()
  const { items: visible, total } = useFilteredItems(items, { kinds: ['task'], categories: filters.categories })
  const tasks = items.filter((item) => item.kind === 'task').length

  return (
    <div className="flex flex-col gap-4 p-6">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h1 className="text-xl font-semibold">Tasks</h1>
          <p className="text-sm text-muted-foreground">
            {isPending ? 'Loading…' : `${visible.length} of ${tasks} tasks`}
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <ItemFiltersBar />
          <QueryField count={visible.length} total={total} />
        </div>
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
