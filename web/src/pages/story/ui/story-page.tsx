import { useMemo } from 'react'
import { useParams } from 'react-router'
import { progressOf, ProgressMeter, useItems } from '@/entities/item'
import { useSchema } from '@/entities/schema'
import { scopeItems, useItemFilters } from '@/features/filter-items'
import type { Category, ItemDetail } from '@/shared/api'
import { referenceFromSplat } from '@/shared/config'
import { ItemView } from '@/widgets/item-view'
import { storyTasks } from '../model/story-tasks'
import { StoryTasksTable } from './story-tasks-table'

function StoryProgress({ detail }: { detail: ItemDetail }) {
  const { data: items = [] } = useItems()
  const { data: schema } = useSchema()
  const { filters } = useItemFilters()
  const labels = (category: Category) => schema?.categories.find((entry) => entry.id === category)?.label ?? category
  const children = useMemo(
    () => scopeItems(detail.relations.children, { node: null, includeArchived: filters.includeArchived }),
    [detail.relations.children, filters.includeArchived],
  )
  const rows = useMemo(() => storyTasks(children, items), [children, items])
  const progress = progressOf(children)
  const ready = rows.filter((row) => row.ready).length
  return (
    <section className="space-y-4">
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
      <StoryTasksTable rows={rows} scope={detail.node} />
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
