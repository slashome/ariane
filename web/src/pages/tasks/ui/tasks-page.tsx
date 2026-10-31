import { useItems } from '@/entities/item'
import { ItemFiltersBar, QueryField, useFilteredItems, useItemFilters } from '@/features/filter-items'
import { ItemsTable } from './items-table'
import './tasks.css'

export function TasksPage() {
  const { data: items = [], isPending, error } = useItems()
  const { filters } = useItemFilters()
  const { items: visible, total } = useFilteredItems(items, { kinds: ['task'], categories: filters.categories })
  const tasks = items.filter((item) => item.kind === 'task').length

  return (
    <div className="tasks-page">
      <div className="tasks-page__header">
        <div>
          <h1 className="tasks-page__title">Tasks</h1>
          <p className="tasks-page__description">
            {isPending ? 'Loading…' : `${visible.length} of ${tasks} tasks`}
          </p>
        </div>
        <div className="tasks-page__actions">
          <ItemFiltersBar />
          <QueryField count={visible.length} total={total} />
        </div>
      </div>
      {error ? (
        <p role="alert" className="tasks-page__error">
          Could not load the items: {error.message}
        </p>
      ) : (
        <ItemsTable items={visible} />
      )}
    </div>
  )
}
