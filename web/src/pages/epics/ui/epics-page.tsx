import { useMemo } from 'react'
import { idColumn, isClosed, progressColumn, statusColumn, titleColumn, useItems } from '@/entities/item'
import { statusLabel, useNodes } from '@/entities/node'
import { useSchema } from '@/entities/schema'
import { QueryField, scopeItems, useFilteredItems, useItemFilters } from '@/features/filter-items'
import type { Category } from '@/shared/api'
import { cn, displayNode } from '@/shared/lib'
import { DataTable, EmptyState, ErrorState, LoadingState, PageHeader, type Column } from '@/shared/ui'
import { epicRows, type EpicRow } from '../model/epic-rows'

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

  const epicOf = (row: EpicRow) => row.epic
  const columns: Column<EpicRow>[] = [
    idColumn(epicOf),
    titleColumn(epicOf, {
      header: 'Epic',
      details: ({ epic, stories, tasks }) => (
        <>
          {epic.node !== filters.node && <span className="font-mono">{displayNode(epic.node)}</span>}
          <span>
            {stories.length} {stories.length === 1 ? 'story' : 'stories'} · {tasks.length} {tasks.length === 1 ? 'task' : 'tasks'}
          </span>
        </>
      ),
    }),
    statusColumn(epicOf, (epic) => statusLabel(nodes, epic.node, epic.status)),
    progressColumn('stories', 'Stories', (row) => row.storyProgress, labels, 'no story'),
    progressColumn('tasks', 'Tasks of its stories', (row) => row.taskProgress, labels),
  ]

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
        <EmptyState icon="layers" title={total === 0 ? 'No epic here' : 'No epic matches the search'}>
          {total === 0 ? (
            <>
              An epic is a Markdown file under <code>epics/</code>; its stories point to it with <code>parent</code>.
            </>
          ) : (
            `${total} epics in scope; the search field keeps none of them.`
          )}
        </EmptyState>
      ) : (
        <DataTable rows={rows} columns={columns} rowKey={(row) => row.epic.path} storageKey="epics" label="Epics" rowProps={(row) => ({ className: cn('align-middle', isClosed(row.epic.category) && 'opacity-70') })} />
      )}
    </div>
  )
}
