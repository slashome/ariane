import { BookOpen } from 'lucide-react'
import { useMemo } from 'react'
import { useSearchParams } from 'react-router'
import { isClosed, ItemLink, ProgressMeter, StatusBadge, useItems } from '@/entities/item'
import { statusLabel, useNodes } from '@/entities/node'
import { useSchema } from '@/entities/schema'
import { emptyFilters, filterItems, scopeItems, useItemFilters } from '@/features/filter-items'
import type { Category } from '@/shared/api'
import { cn, displayNode } from '@/shared/lib'
import { EmptyState, ErrorState, LoadingState, NativeSelect, PageHeader } from '@/shared/ui'
import { groupings, groupStories, type StoryGrouping } from '../model/story-groups'

const groupParam = 'group'

export function StoriesPage() {
  const { data: items = [], isPending, error } = useItems()
  const { data: nodes = [] } = useNodes()
  const { data: schema } = useSchema()
  const { filters } = useItemFilters()
  const [params, setParams] = useSearchParams()
  const grouping = (groupings.find((entry) => entry.value === params.get(groupParam))?.value ?? 'milestone') as StoryGrouping
  const labels = (category: Category) => schema?.categories.find((entry) => entry.id === category)?.label ?? category

  const groups = useMemo(() => {
    const stories = filterItems(items, {
      ...emptyFilters,
      node: filters.node,
      includeArchived: filters.includeArchived,
      kinds: ['story'],
      query: filters.query,
    })
    const tasks = scopeItems(items, { node: null, includeArchived: filters.includeArchived })
    const statusLabel = (key: string) => schema?.categories.find((entry) => entry.id === key)?.label ?? key
    return groupStories(stories, tasks, grouping, grouping === 'status' ? statusLabel : undefined)
  }, [items, filters, grouping, schema])
  const count = groups.reduce((sum, group) => sum + group.items.length, 0)

  const setGrouping = (value: string) =>
    setParams(
      (current) => {
        const next = new URLSearchParams(current)
        next.set(groupParam, value)
        return next
      },
      { replace: true },
    )

  return (
    <div className="flex flex-col gap-5 p-6">
      <PageHeader
        title="Stories"
        description={isPending ? 'Loading…' : `${count} ${count === 1 ? 'story' : 'stories'} — progress counts the tasks whose parent is the story, dropped ones aside`}
        actions={
          <label className="flex items-center gap-2 text-sm text-muted-foreground">
            Group by
            <NativeSelect aria-label="Group stories by" value={grouping} onChange={(event) => setGrouping(event.target.value)}>
              {groupings.map((entry) => (
                <option key={entry.value} value={entry.value}>
                  {entry.label}
                </option>
              ))}
            </NativeSelect>
          </label>
        }
      />
      {error ? (
        <ErrorState message={`Could not load the items: ${error.message}`} />
      ) : isPending ? (
        <LoadingState />
      ) : count === 0 ? (
        <EmptyState icon={BookOpen} title="No story here">
          A story is a Markdown file under <code>stories/</code>; its tasks point to it with <code>parent</code>.
        </EmptyState>
      ) : (
        groups.map((group) => (
          <section key={group.key} aria-labelledby={`group-${group.key}`} className="space-y-2">
            <div className="flex flex-wrap items-center gap-x-4 gap-y-1 border-b border-border pb-2">
              <h2 id={`group-${group.key}`} className="text-sm font-semibold">
                {group.label} <span className="font-normal text-muted-foreground">({group.items.length})</span>
              </h2>
              <ProgressMeter progress={group.progress} labels={labels} className="w-72 max-w-full" />
            </div>
            <ul className="divide-y divide-border">
              {group.items.map(({ story, tasks, progress }) => (
                <li key={story.path} className={cn('grid items-center gap-x-4 gap-y-1 py-2.5 md:grid-cols-[minmax(0,1fr)_9rem_18rem]', isClosed(story.category) && 'opacity-70')}>
                  <div className="min-w-0">
                    <div className="flex items-baseline gap-2">
                      <span className="shrink-0 font-mono text-xs text-muted-foreground">{story.id}</span>
                      <ItemLink item={story} className="truncate font-medium">
                        {story.title ?? story.path}
                      </ItemLink>
                    </div>
                    <p className="mt-0.5 flex flex-wrap gap-x-3 text-xs text-muted-foreground">
                      {story.node !== filters.node && <span className="font-mono">{displayNode(story.node)}</span>}
                      {grouping !== 'milestone' && story.milestone && <span>milestone {story.milestone}</span>}
                      {grouping !== 'milestone' && grouping !== 'release' && story.release && <span>release {story.release}</span>}
                      <span>
                        {tasks.length} {tasks.length === 1 ? 'task' : 'tasks'}
                      </span>
                    </p>
                  </div>
                  <div>
                    <StatusBadge status={story.status} category={story.category} label={statusLabel(nodes, story.node, story.status)} />
                  </div>
                  <ProgressMeter progress={progress} labels={labels} />
                </li>
              ))}
            </ul>
          </section>
        ))
      )}
    </div>
  )
}
