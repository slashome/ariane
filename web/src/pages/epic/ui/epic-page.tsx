import { useMemo } from 'react'
import { useParams } from 'react-router'
import {
  childrenOf,
  epicProgress,
  idColumn,
  isClosed,
  progressColumn,
  progressOf,
  ProgressMeter,
  statusColumn,
  titleColumn,
  useItems,
  type Progress,
} from '@/entities/item'
import { statusLabel, useNodes } from '@/entities/node'
import { useSchema } from '@/entities/schema'
import { QueryField, scopeItems, useFilteredItems, useItemFilters } from '@/features/filter-items'
import type { Category, Item, ItemDetail } from '@/shared/api'
import { referenceFromSplat } from '@/shared/config'
import { cn, displayNode } from '@/shared/lib'
import { DataTable, EmptyState, type Column } from '@/shared/ui'
import { ItemView } from '@/widgets/item-view'

interface EpicStoryRow {
  story: Item
  tasks: Item[]
  progress: Progress
}

function EpicProgressPanel({ detail }: { detail: ItemDetail }) {
  const { data: items = [] } = useItems()
  const { data: nodes = [] } = useNodes()
  const { data: schema } = useSchema()
  const { filters } = useItemFilters()
  const labels = (category: Category) => schema?.categories.find((entry) => entry.id === category)?.label ?? category
  const scoped = useMemo(
    () => scopeItems(items, { node: null, includeArchived: filters.includeArchived, includeDescendants: false }),
    [items, filters.includeArchived],
  )
  const { stories, storyProgress, taskProgress } = useMemo(() => epicProgress(detail, scoped), [detail, scoped])
  const { items: matching, total } = useFilteredItems(stories, { anyNode: true })
  const rows = useMemo<EpicStoryRow[]>(
    () =>
      matching.map((story) => {
        const tasks = childrenOf(scoped, story.reference)
        return { story, tasks, progress: progressOf(tasks) }
      }),
    [matching, scoped],
  )
  const storyOf = (row: EpicStoryRow) => row.story
  const columns: Column<EpicStoryRow>[] = [
    idColumn(storyOf),
    titleColumn(storyOf, {
      header: 'Story',
      details: ({ story, tasks }) => (
        <>
          {story.node !== detail.node && <span className="font-mono">{displayNode(story.node)}</span>}
          <span>
            {tasks.length} {tasks.length === 1 ? 'task' : 'tasks'}
          </span>
        </>
      ),
    }),
    statusColumn(storyOf, (story) => statusLabel(nodes, story.node, story.status)),
    progressColumn('progress', 'Tasks', (row) => row.progress, labels),
  ]
  return (
    <section className="space-y-4">
      <div className="space-y-4 rounded-lg border border-border p-4">
        <div className="space-y-2">
          <div className="flex flex-wrap items-baseline justify-between gap-2">
            <h2 className="text-sm font-semibold">Stories</h2>
            {!filters.includeArchived && <p className="text-xs text-muted-foreground">archived stories and tasks hidden</p>}
          </div>
          <ProgressMeter progress={storyProgress} labels={labels} empty="no story" legend />
        </div>
        <div className="space-y-2">
          <h2 className="text-sm font-semibold">Tasks of its stories</h2>
          <ProgressMeter progress={taskProgress} labels={labels} />
        </div>
      </div>
      <div className="flex justify-end">
        <QueryField count={matching.length} total={total} />
      </div>
      {matching.length === 0 ? (
        total > 0 ? (
          <EmptyState title="No story of this epic matches the search">{total} stories in the epic; the search field keeps none of them.</EmptyState>
        ) : (
          <EmptyState title="No story in this epic yet">A story joins it with `parent:` set to this epic.</EmptyState>
        )
      ) : (
        <DataTable
          rows={rows}
          columns={columns}
          rowKey={(row) => row.story.path}
          storageKey="epic-stories"
          label="Stories of the epic"
          rowProps={(row) => ({ className: cn('align-middle', isClosed(row.story.category) && 'opacity-70') })}
        />
      )}
    </section>
  )
}

export function EpicPage() {
  const { '*': splat } = useParams()
  return (
    <div className="p-6">
      <ItemView reference={referenceFromSplat(splat)} sectionLabel="Epics">
        {(detail) => <EpicProgressPanel detail={detail} />}
      </ItemView>
    </div>
  )
}
