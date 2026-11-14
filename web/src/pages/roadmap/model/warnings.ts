import type { ForecastWarningKind, Roadmap } from '@/shared/api'
import type { GanttRow } from './rows'

export const warningKinds: readonly ForecastWarningKind[] = [
  'missed_target',
  'contradictory_target',
  'dependency_cycle',
  'unscheduled_dependency',
  'undeclared_release',
  'without_release',
  'without_epic',
  'without_story',
  'unsized',
  'provisional',
]

export const warningLabels: Record<ForecastWarningKind, string> = {
  unsized: 'Unsized',
  provisional: 'Provisional size',
  without_story: 'No story',
  without_epic: 'No epic',
  without_release: 'No release',
  undeclared_release: 'Undeclared release',
  missed_target: 'Missed target',
  contradictory_target: 'Contradictory target',
  dependency_cycle: 'Dependency cycle',
  unscheduled_dependency: 'Unscheduled dependency',
}

export const warningHints: Record<ForecastWarningKind, string> = {
  unsized: 'Scheduled tasks without a size, counted at the default size',
  provisional: 'Stories and epics not split yet, weighed by their provisional or the default size',
  without_story: 'Open tasks that belong to no story',
  without_epic: 'Stories with open work that belong to no epic',
  without_release: 'Open work with no effective release: left out of the forecast',
  undeclared_release: 'Open work whose release no list of releases declares: left out of the forecast',
  missed_target: 'Work, stories, epics or releases forecast after their target date',
  contradictory_target: 'A target later than the target of the story, epic or release containing it',
  dependency_cycle: 'Work caught in a dependency cycle: left unscheduled',
  unscheduled_dependency: 'Work depending on open work that is never scheduled: the dependency is ignored',
}

export interface WarningCounter {
  kind: ForecastWarningKind
  label: string
  hint: string
  count: number
  muted: boolean
}

export const mutedHint = 'Muted: it touches more than half of the scheduled work, so it says little about any one item'

export function scheduledWork(roadmap: Pick<Roadmap, 'releases'>): Set<string> {
  return new Set(
    roadmap.releases.flatMap((release) => release.epics.flatMap((epic) => epic.stories.flatMap((story) => story.work.map((work) => work.reference)))),
  )
}

export function touchesMostOf(flagged: ReadonlySet<string>, scheduled: ReadonlySet<string>): boolean {
  const touched = [...flagged].filter((reference) => scheduled.has(reference)).length
  return touched * 2 > scheduled.size
}

export function warningCounters(roadmap: Pick<Roadmap, 'warnings' | 'releases'>): WarningCounter[] {
  const counts = new Map<ForecastWarningKind, number>()
  for (const warning of roadmap.warnings) {
    if (warning.count > 0) {
      counts.set(warning.kind, (counts.get(warning.kind) ?? 0) + warning.count)
    }
  }
  const scheduled = scheduledWork(roadmap)
  return warningKinds.flatMap((kind) => {
    const count = counts.get(kind)
    if (count === undefined) {
      return []
    }
    const muted = touchesMostOf(flaggedBy(roadmap, kind) ?? new Set(), scheduled)
    return [{ kind, label: warningLabels[kind], hint: muted ? `${warningHints[kind]}. ${mutedHint}` : warningHints[kind], count, muted }]
  })
}

export function flaggedBy(roadmap: Pick<Roadmap, 'warnings'>, kind: ForecastWarningKind | null): Set<string> | null {
  if (kind === null) {
    return null
  }
  return new Set(roadmap.warnings.filter((warning) => warning.kind === kind).flatMap((warning) => warning.items.map((item) => item.reference)))
}

export function filterRows(rows: GanttRow[], flagged: ReadonlySet<string> | null): GanttRow[] {
  if (flagged === null) {
    return rows
  }
  return rows.filter((row) => row.covers.some((reference) => flagged.has(reference)))
}
