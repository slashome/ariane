import { Layers } from 'lucide-react'
import { useMemo } from 'react'
import { isClosed, ItemLink, ProgressMeter, StatusBadge, useItems } from '@/entities/item'
import { statusLabel, useNodes } from '@/entities/node'
import { useSchema } from '@/entities/schema'
import { QueryField, scopeItems, useFilteredItems, useItemFilters } from '@/features/filter-items'
import type { Category } from '@/shared/api'
import { cn, displayNode } from '@/shared/lib'
import { EmptyState, ErrorState, Highlight, LoadingState, PageHeader } from '@/shared/ui'
import { epicRows } from '../model/epic-rows'

export function EpicsPage() {
  const { data: items = [], isPending, error } = useItems()
  const { data: nodes = [] } = useNodes()
  const { data: schema } = useSchema()
  const { filters } = useItemFilters()
  const labels = (category: Category) => schema?.categories.find((entry) => entry.id === category)?.label ?? category

  const { items: epics, total } = useFilteredItems(items, { kinds: ['epic'] })
  const rows = useMemo(() => {
    const children = scopeItems(items, { node: null, includeArchived: filters.includeArchived, includeDescendants: false })
    return epicRows(epics, children)
  }, [items, epics, filters.includeArchived])

  return (
    <div className="flex flex-col gap-5 p-6">
      <PageHeader
        title="Epics"
        description={
          isPending
            ? 'Loading…'
            : `${rows.length} ${rows.length === 1 ? 'epic' : 'epics'} — progress counts the stories whose parent is the epic, then their tasks, dropped ones aside`
        }
        actions={<QueryField count={rows.length} total={total} />}
      />
      {error ? (
        <ErrorState message={`Could not load the items: ${error.message}`} />
      ) : isPending ? (
        <LoadingState />
      ) : rows.length === 0 ? (
        <EmptyState icon={Layers} title={total === 0 ? 'No epic here' : 'No epic matches the search'}>
          {total === 0 ? (
            <>
              An epic is a Markdown file under <code>epics/</code>; its stories point to it with <code>parent</code>.
            </>
          ) : (
            `${total} epics in scope; the search field keeps none of them.`
          )}
        </EmptyState>
      ) : (
        <ul className="divide-y divide-border">
          <li aria-hidden className="hidden gap-x-4 pb-2 text-xs text-muted-foreground md:grid md:grid-cols-[minmax(0,1fr)_9rem_16rem_16rem]">
            <span>Epic</span>
            <span>Status</span>
            <span>Stories</span>
            <span>Tasks of its stories</span>
          </li>
          {rows.map(({ epic, stories, tasks, storyProgress, taskProgress }) => (
            <li
              key={epic.path}
              className={cn(
                'grid items-center gap-x-4 gap-y-1 py-2.5 md:grid-cols-[minmax(0,1fr)_9rem_16rem_16rem]',
                isClosed(epic.category) && 'opacity-70',
              )}
            >
              <div className="min-w-0">
                <div className="flex items-baseline gap-2">
                  <span className="shrink-0 font-mono text-xs text-muted-foreground">
                    <Highlight text={epic.id ?? ''} />
                  </span>
                  <ItemLink item={epic} className="truncate font-medium">
                    <Highlight text={epic.title ?? epic.path} />
                  </ItemLink>
                </div>
                <p className="mt-0.5 flex flex-wrap gap-x-3 text-xs text-muted-foreground">
                  {epic.node !== filters.node && <span className="font-mono">{displayNode(epic.node)}</span>}
                  <span>
                    {stories.length} {stories.length === 1 ? 'story' : 'stories'} · {tasks.length} {tasks.length === 1 ? 'task' : 'tasks'}
                  </span>
                </p>
              </div>
              <div>
                <StatusBadge status={epic.status} category={epic.category} label={statusLabel(nodes, epic.node, epic.status)} />
              </div>
              <ProgressMeter progress={storyProgress} labels={labels} empty="no story" />
              <ProgressMeter progress={taskProgress} labels={labels} />
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}
