import { useMemo, useState } from 'react'
import { useItems } from '@/entities/item'
import { useItemFilters } from '@/features/filter-items'
import type { Roadmap } from '@/shared/api'
import { searchParamKeys } from '@/shared/config'
import { displayNode, useSearchParam } from '@/shared/lib'
import { ErrorState, PageHeader } from '@/shared/ui'
import { useRoadmap } from '../api/use-roadmap'
import { useGanttLevels } from '../model/levels'
import { buildRows } from '../model/rows'
import { useElementWidth, useGanttView } from '../model/view'
import { GanttChart } from './gantt-chart'
import { GanttNav } from './gantt-nav'
import { LevelToggle } from './level-toggle'

function describe(node: string | null, roadmap: Roadmap | undefined, isPending: boolean): string {
  if (node === null || node === '') {
    return 'Choose a node in the header to see its roadmap'
  }
  if (roadmap === undefined) {
    return isPending ? `${displayNode(node)} — loading…` : displayNode(node)
  }
  const timing = roadmap.carrier === null ? 'no calendar: focused hours from now' : `dates from the calendar of ${displayNode(roadmap.carrier)}`
  return `${displayNode(node)} — ${timing}`
}

export function GanttPage() {
  const { filters } = useItemFilters()
  const node = filters.node
  const { data: roadmap, isPending, error } = useRoadmap(node)
  const { data: items = [] } = useItems()
  const [levels, toggleLevel] = useGanttLevels()
  const [width, measure] = useElementWidth()
  const view = useGanttView(roadmap?.dated ?? false, width)
  const [, setPreview] = useSearchParam(searchParamKeys.preview)
  const [opened] = useState(() => new Date().toISOString())

  const rows = useMemo(() => (roadmap ? buildRows(roadmap, levels) : []), [roadmap, levels])
  const paths = useMemo(() => new Map(items.map((item) => [`${item.node}#${item.id}`, item.path])), [items])
  const open = (reference: string) => {
    const path = paths.get(reference)
    if (path) {
      setPreview(path)
    }
  }
  const chosen = node !== null && node !== ''

  return (
    <div className="flex h-full flex-col gap-4 p-6">
      <PageHeader
        title="Gantt"
        description={describe(node, roadmap, isPending)}
        actions={
          <>
            <GanttNav view={view} />
            <LevelToggle active={levels} onToggle={toggleLevel} />
          </>
        }
      />
      {error && <ErrorState message={`Could not load the roadmap: ${error.message}`} />}
      <GanttChart
        rows={rows}
        view={view}
        measure={measure}
        now={roadmap?.now ?? opened}
        empty={!chosen ? 'No node chosen.' : isPending ? 'Loading…' : 'No release to show.'}
        onOpen={open}
      />
    </div>
  )
}
