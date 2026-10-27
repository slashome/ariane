import { useMemo } from 'react'
import { useParams } from 'react-router'
import { childrenOf, epicProgress, isClosed, ItemLink, progressOf, ProgressMeter, StatusBadge, useItems } from '@/entities/item'
import { statusLabel, useNodes } from '@/entities/node'
import { useSchema } from '@/entities/schema'
import { QueryField, scopeItems, useFilteredItems, useItemFilters } from '@/features/filter-items'
import type { Category, ItemDetail } from '@/shared/api'
import { referenceFromSplat } from '@/shared/config'
import { cn, displayNode } from '@/shared/lib'
import { EmptyState, Highlight } from '@/shared/ui'
import { ItemView } from '@/widgets/item-view'

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
        <ul className="divide-y divide-border rounded-lg border border-border">
          {matching.map((story) => {
            const tasks = childrenOf(scoped, story.reference)
            return (
              <li
                key={story.path}
                className={cn(
                  'grid items-center gap-x-4 gap-y-1 px-3 py-2.5 md:grid-cols-[minmax(0,1fr)_9rem_16rem]',
                  isClosed(story.category) && 'opacity-70',
                )}
              >
                <div className="min-w-0">
                  <div className="flex items-baseline gap-2">
                    <span className="shrink-0 font-mono text-xs text-muted-foreground">
                      <Highlight text={story.id ?? ''} />
                    </span>
                    <ItemLink item={story} className="truncate font-medium">
                      <Highlight text={story.title ?? story.path} />
                    </ItemLink>
                  </div>
                  <p className="mt-0.5 flex flex-wrap gap-x-3 text-xs text-muted-foreground">
                    {story.node !== detail.node && <span className="font-mono">{displayNode(story.node)}</span>}
                    <span>
                      {tasks.length} {tasks.length === 1 ? 'task' : 'tasks'}
                    </span>
                  </p>
                </div>
                <div>
                  <StatusBadge status={story.status} category={story.category} label={statusLabel(nodes, story.node, story.status)} />
                </div>
                <ProgressMeter progress={progressOf(tasks)} labels={labels} />
              </li>
            )
          })}
        </ul>
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
