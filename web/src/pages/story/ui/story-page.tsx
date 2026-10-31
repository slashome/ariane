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
import './story.css'

function StoryPersonas({ detail }: { detail: ItemDetail }) {
  const { data: nodes = [] } = useNodes()
  const personas = personasOf(nodes, detail.node, detail.personas)
  if (personas.length === 0) {
    return null
  }
  return (
    <section className="story-personas" aria-label="Personas">
      <h2 className="story-personas__title">Written for</h2>
      {personas.map((persona) => (
        <span key={persona.id} title={persona.id} className="story-personas__persona">
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
    <section className="story-epic" aria-label="Epic">
      <h2 className="story-epic__title">Part of</h2>
      <span className="story-epic__id">{epic.id}</span>
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
    <section className="story-progress">
      <StoryEpic detail={detail} />
      <StoryPersonas detail={detail} />
      <div className="story-progress__panel">
        <div className="story-progress__header">
          <h2 className="story-progress__title">Progress</h2>
          <p className="story-progress__summary">
            {ready} ready to start · {progress.counts.blocked} blocked
            {!filters.includeArchived && ' · archived tasks hidden'}
          </p>
        </div>
        <ProgressMeter progress={progress} labels={labels} legend />
      </div>
      <div className="story-progress__query">
        <QueryField count={rows.length} total={total} />
      </div>
      <StoryTasksTable rows={rows} scope={detail.node} hidden={total - rows.length} />
    </section>
  )
}

export function StoryPage() {
  const { '*': splat } = useParams()
  return (
    <div className="story-page">
      <ItemView reference={referenceFromSplat(splat)} sectionLabel="Stories">
        {(detail) => <StoryProgress detail={detail} />}
      </ItemView>
    </div>
  )
}
