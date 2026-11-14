import { useMemo, useState } from 'react'
import { useItems, useRankConfirmation } from '@/entities/item'
import { useNodes } from '@/entities/node'
import { useItemFilters } from '@/features/filter-items'
import type { Estimate, ForecastWarningKind, Roadmap } from '@/shared/api'
import { searchParamKeys } from '@/shared/config'
import { displayNode, useSearchParam } from '@/shared/lib'
import { ErrorState, PageHeader } from '@/shared/ui'
import { useGanttEdits } from '../api/use-gantt-edits'
import { useRoadmap } from '../api/use-roadmap'
import { buildColumns, narrowColumns, useUnfoldedColumns } from '../model/columns'
import { useGanttLevels } from '../model/levels'
import { buildRows } from '../model/rows'
import { useElementWidth, useGanttView } from '../model/view'
import { filterRows, flaggedBy, warningCounters } from '../model/warnings'
import { CalendarBanner } from './calendar-banner'
import { GanttChart, type GanttEditing } from './gantt-chart'
import { GanttNav } from './gantt-nav'
import { LevelToggle } from './level-toggle'
import { UndatedColumns } from './undated-columns'
import { WarningsBanner } from './warnings-banner'

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

function emptyMessage(chosen: boolean, isPending: boolean, filtered: boolean): string {
  if (!chosen) {
    return 'No node chosen.'
  }
  if (isPending) {
    return 'Loading…'
  }
  return filtered ? 'Nothing scheduled carries this warning: see the columns on the right.' : 'No release to show.'
}

export function GanttPage() {
  const { filters } = useItemFilters()
  const node = filters.node
  const { data: roadmap, isPending, error } = useRoadmap(node)
  const { data: items = [] } = useItems()
  const { data: nodes = [] } = useNodes()
  const edits = useGanttEdits()
  const ranking = useRankConfirmation(items)
  const [levels, toggleLevel] = useGanttLevels()
  const [unfolded, toggleColumn] = useUnfoldedColumns()
  const [width, measure] = useElementWidth()
  const view = useGanttView(roadmap?.dated ?? false, width)
  const [, setPreview] = useSearchParam(searchParamKeys.preview)
  const [opened] = useState(() => new Date().toISOString())
  const [chosenWarning, setChosenWarning] = useState<ForecastWarningKind | null>(null)

  const counters = useMemo(() => (roadmap ? warningCounters(roadmap) : []), [roadmap])
  const warning = counters.some((counter) => counter.kind === chosenWarning) ? chosenWarning : null
  const flagged = useMemo(() => (roadmap ? flaggedBy(roadmap, warning) : null), [roadmap, warning])
  const rows = useMemo(() => (roadmap ? filterRows(buildRows(roadmap, levels), flagged) : []), [roadmap, levels, flagged])
  const columns = useMemo(() => (roadmap ? narrowColumns(buildColumns(roadmap), flagged) : []), [roadmap, flagged])
  const byReference = useMemo(() => new Map(items.map((item) => [`${item.node}#${item.id}`, item])), [items])
  const paths = useMemo(() => new Map([...byReference].map(([reference, item]) => [reference, item.path])), [byReference])
  const pricing = useMemo(() => new Map(nodes.map((entry) => [entry.path, entry.estimates])), [nodes])
  const editing: GanttEditing = {
    itemOf: (row) => (row.reference === null ? undefined : byReference.get(row.reference)),
    pricingOf: (item) => pricing.get(item.node),
    move: (item, other, place) => ranking.confirm(item, place, other, () => edits.move(item, other, place)),
    resize: edits.resize,
  }
  const estimates = useMemo(
    () =>
      new Map(
        items.flatMap((item): [string, Estimate][] => (item.estimate === null ? [] : [[`${item.node}#${item.id}`, item.estimate]])),
      ),
    [items],
  )
  const open = (reference: string) => {
    const path = paths.get(reference)
    if (path) {
      setPreview(path)
    }
  }
  const chosen = node !== null && node !== ''

  return (
    <div className="flex h-full flex-col gap-3 p-6">
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
      {roadmap && roadmap.carrier === null && <CalendarBanner />}
      <WarningsBanner
        counters={counters}
        active={warning}
        onToggle={(kind) => setChosenWarning(kind === warning ? null : kind)}
      />
      <div className="flex min-h-0 flex-1 gap-2">
        <GanttChart
          rows={rows}
          view={view}
          measure={measure}
          now={roadmap?.now ?? opened}
          empty={emptyMessage(chosen, isPending, warning !== null)}
          onOpen={open}
          editing={editing}
        />
        <UndatedColumns columns={columns} unfolded={unfolded} estimates={estimates} onToggle={toggleColumn} onOpen={open} />
      </div>
      {ranking.dialog}
    </div>
  )
}
