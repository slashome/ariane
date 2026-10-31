import { useMemo } from 'react'
import { useSearchParams } from 'react-router'
import { idColumn, isClosed, progressColumn, ProgressMeter, statusColumn, titleColumn, useItems } from '@/entities/item'
import { statusLabel, useNodes } from '@/entities/node'
import { useSchema } from '@/entities/schema'
import { QueryField, scopeItems, useFilteredItems, useItemFilters } from '@/features/filter-items'
import type { Category } from '@/shared/api'
import { cn, displayNode } from '@/shared/lib'
import { DataTable, EmptyState, ErrorState, LoadingState, NativeSelect, PageHeader, type Column } from '@/shared/ui'
import { groupings, groupStories, type StoryGrouping, type StoryRow } from '../model/story-groups'
import './stories.css'

const groupParam = 'group'

export function StoriesPage() {
  const { data: items = [], isPending, error } = useItems()
  const { data: nodes = [] } = useNodes()
  const { data: schema } = useSchema()
  const { filters } = useItemFilters()
  const [params, setParams] = useSearchParams()
  const grouping = (groupings.find((entry) => entry.value === params.get(groupParam))?.value ?? 'milestone') as StoryGrouping
  const labels = (category: Category) => schema?.categories.find((entry) => entry.id === category)?.label ?? category

  const { items: stories, total } = useFilteredItems(items, { kinds: ['story'] })
  const groups = useMemo(() => {
    const tasks = scopeItems(items, { node: null, includeArchived: filters.includeArchived, includeDescendants: false })
    const statusLabel = (key: string) => schema?.categories.find((entry) => entry.id === key)?.label ?? key
    const epicLabel = (key: string) => {
      const epic = items.find((item) => item.reference === key)
      return epic ? `${epic.id} — ${epic.title ?? epic.path}` : key
    }
    const label = grouping === 'status' ? statusLabel : grouping === 'epic' ? epicLabel : undefined
    return groupStories(stories, tasks, grouping, label)
  }, [items, stories, filters.includeArchived, grouping, schema])
  const count = groups.reduce((sum, group) => sum + group.items.length, 0)

  const storyOf = (row: StoryRow) => row.story
  const columns: Column<StoryRow>[] = [
    idColumn(storyOf),
    titleColumn(storyOf, {
      header: 'Story',
      details: ({ story, tasks }) => (
        <>
          {story.node !== filters.node && <span className="stories-page__node">{displayNode(story.node)}</span>}
          {grouping !== 'milestone' && story.milestone && <span>milestone {story.milestone}</span>}
          {grouping !== 'milestone' && grouping !== 'release' && story.release && <span>release {story.release}</span>}
          <span>
            {tasks.length} {tasks.length === 1 ? 'task' : 'tasks'}
          </span>
        </>
      ),
    }),
    statusColumn(storyOf, (story) => statusLabel(nodes, story.node, story.status)),
    progressColumn('progress', 'Tasks', (row) => row.progress, labels),
  ]

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
    <div className="stories-page">
      <PageHeader
        title="Stories"
        description={isPending ? 'Loading…' : `${count} ${count === 1 ? 'story' : 'stories'} — progress counts the tasks whose parent is the story, dropped ones aside`}
        actions={
          <>
          <QueryField count={count} total={total} />
          <label className="stories-page__grouping">
            Group by
            <NativeSelect aria-label="Group stories by" value={grouping} onChange={(event) => setGrouping(event.target.value)}>
              {groupings.map((entry) => (
                <option key={entry.value} value={entry.value}>
                  {entry.label}
                </option>
              ))}
            </NativeSelect>
          </label>
          </>
        }
      />
      {error ? (
        <ErrorState message={`Could not load the items: ${error.message}`} />
      ) : isPending ? (
        <LoadingState />
      ) : count === 0 ? (
        <EmptyState icon="book" title={total === 0 ? 'No story here' : 'No story matches the search'}>
          {total === 0 ? (
            <>
              A story is a Markdown file under <code>stories/</code>; its tasks point to it with <code>parent</code>.
            </>
          ) : (
            `${total} stories in scope; the search field keeps none of them.`
          )}
        </EmptyState>
      ) : (
        groups.map((group) => (
          <section key={group.key} aria-labelledby={`group-${group.key}`} className="stories-group">
            <div className="stories-group__header">
              <h2 id={`group-${group.key}`} className="stories-group__title">
                {group.label} <span className="stories-group__count">({group.items.length})</span>
              </h2>
              <ProgressMeter progress={group.progress} labels={labels} className="stories-group__meter" />
            </div>
            <DataTable
              rows={group.items}
              columns={columns}
              rowKey={(row) => row.story.path}
              storageKey="stories"
              label={group.label}
              rowProps={(row) => ({ className: cn('stories-group__row', isClosed(row.story.category) && 'stories-group__row--closed') })}
            />
          </section>
        ))
      )}
    </div>
  )
}
