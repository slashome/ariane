import { useMemo } from 'react'
import { useParams } from 'react-router'
import { ItemLink, progressOf, ProgressMeter, useItems } from '@/entities/item'
import { personasOf, useNodes } from '@/entities/node'
import { useSchema } from '@/entities/schema'
import { QueryField, scopeItems, useFilteredItems, useItemFilters } from '@/features/filter-items'
import type { Category, ItemDetail } from '@/shared/api'
import { referenceFromSplat } from '@/shared/config'
import { ItemView } from '@/widgets/item-view'
import { storyTasks } from '../model/story-tasks'
import { StoryTasksTable } from './story-tasks-table'

function StoryPersonas({ detail }: { detail: ItemDetail }) {
  const { data: nodes = [] } = useNodes()
  const personas = personasOf(nodes, detail.node, detail.personas)
  if (personas.length === 0) {
    return null
  }
  return (
    <section className="flex flex-wrap items-center gap-2" aria-label="Personas">
      <h2 className="text-sm font-semibold">Written for</h2>
      {personas.map((persona) => (
        <span key={persona.id} title={persona.id} className="rounded-full border border-border px-2 py-0.5 text-xs">
          {persona.name}
        </span>
      ))}
    </section>
  )
}

function StoryEpic({ detail }: { detail: ItemDetail }) {
  const epic = detail.relations.parent
  if (epic?.kind !== 'epic') {
    return null
  }
  return (
    <section className="flex flex-wrap items-baseline gap-2 text-sm" aria-label="Epic">
      <h2 className="font-semibold">Part of</h2>
      <span className="font-mono text-xs text-muted-foreground">{epic.id}</span>
      <ItemLink item={epic}>{epic.title ?? epic.path}</ItemLink>
    </section>
  )
}

function StoryProgress({ detail }: { detail: ItemDetail }) {
  const { data: items = [] } = useItems()
  const { data: schema } = useSchema()
  const { filters } = useItemFilters()
  const labels = (category: Category) => schema?.categories.find((entry) => entry.id === category)?.label ?? category
  const children = useMemo(
    () => scopeItems(detail.relations.children, { node: null, includeArchived: filters.includeArchived, includeDescendants: false }),
    [detail.relations.children, filters.includeArchived],
  )
  const { items: matching, total } = useFilteredItems(children, { anyNode: true })
  const rows = useMemo(() => storyTasks(matching, items), [matching, items])
  const progress = progressOf(children)
  const ready = rows.filter((row) => row.ready).length
  return (
    <section className="space-y-4">
      <StoryEpic detail={detail} />
      <StoryPersonas detail={detail} />
      <div className="space-y-2 rounded-lg border border-border p-4">
        <div className="flex flex-wrap items-baseline justify-between gap-2">
          <h2 className="text-sm font-semibold">Progress</h2>
          <p className="text-xs text-muted-foreground">
            {ready} ready to start · {progress.counts.blocked} blocked
            {!filters.includeArchived && ' · archived tasks hidden'}
          </p>
        </div>
        <ProgressMeter progress={progress} labels={labels} legend />
      </div>
      <div className="flex justify-end">
        <QueryField count={rows.length} total={total} />
      </div>
      <StoryTasksTable rows={rows} scope={detail.node} hidden={total - rows.length} />
    </section>
  )
}

export function StoryPage() {
  const { '*': splat } = useParams()
  return (
    <div className="p-6">
      <ItemView reference={referenceFromSplat(splat)} sectionLabel="Stories">
        {(detail) => <StoryProgress detail={detail} />}
      </ItemView>
    </div>
  )
}
