import type {
  ClosedDate,
  EpicPlan,
  EstimateBasis,
  ForecastSummary,
  Kind,
  Moment,
  ReleasePlan,
  Roadmap,
  StoryPlan,
  Tester,
  WorkPlan,
  WorkStage,
} from '@/shared/api'
import { levelRank, type Level } from './levels'

export type Phase = 'work' | 'test' | 'validation' | 'done'

export interface TimedSegment {
  phase: Phase
  start: number
  end: number
}

export interface WorkBar {
  stage: WorkStage
  hours: number
  testHours: number
  tester: Tester
  basis: EstimateBasis
  hatched: boolean
  segments: TimedSegment[]
  start: Moment | null
  delivered: Moment | null
  closed: ClosedDate | null
}

export interface Piece {
  start: number
  end: number
  done: boolean
  hatched: boolean
}

export interface Fragment {
  release: string
  start: number
  end: number
  pieces: Piece[]
}

export interface GanttRow {
  key: string
  level: Level
  depth: number
  reference: string | null
  kind: Kind | null
  id: string | null
  title: string
  hours: number
  testHours: number
  done: number
  open: number
  target: string | null
  missed: string | null
  fragments: Fragment[]
  work: WorkBar | null
}

const hour = 3_600_000

export function isHatched(work: Pick<WorkPlan, 'basis' | 'stage'>): boolean {
  return work.stage !== 'done' && (work.basis === 'default' || work.basis === 'provisional')
}

function idOf(reference: string | null): string | null {
  if (reference === null) {
    return null
  }
  const at = reference.lastIndexOf('#')
  return at === -1 ? reference : reference.slice(at + 1)
}

function closedDay(date: string, now: number): [number, number] | null {
  const [year, month, day] = date.split('-').map(Number)
  if (!year || !month || !day) {
    return null
  }
  const start = (new Date(year, month - 1, day).getTime() - now) / hour
  const end = Math.min((new Date(year, month - 1, day + 1).getTime() - now) / hour, 0)
  return [start, Math.max(end, start + 1)]
}

export function timeOf(moment: Moment, roadmap: Pick<Roadmap, 'dated' | 'now'>): number | null {
  if (!roadmap.dated) {
    return moment.hours
  }
  return moment.at === null ? null : (Date.parse(moment.at) - Date.parse(roadmap.now)) / hour
}

export function workSegments(work: WorkPlan, roadmap: Pick<Roadmap, 'dated' | 'now'>): TimedSegment[] {
  if (work.stage === 'done') {
    const day = work.closed ? closedDay(work.closed.date, Date.parse(roadmap.now)) : null
    return day ? [{ phase: 'done', start: day[0], end: day[1] }] : []
  }
  const spans: { phase: Phase; start: Moment; end: Moment }[] = [...work.segments]
  if (work.validation) {
    spans.push({ phase: 'validation', start: work.validation.start, end: work.validation.end })
  }
  return spans.flatMap(({ phase, start, end }) => {
    const from = timeOf(start, roadmap)
    const to = timeOf(end, roadmap)
    return from === null || to === null ? [] : [{ phase, start: from, end: to }]
  })
}

interface PlanNode {
  level: Level
  key: string
  reference: string | null
  kind: Kind | null
  title: string
  target: string | null
  release: string
  summary: ForecastSummary | null
  work: WorkPlan | null
  children: PlanNode[]
}

function workNode(work: WorkPlan, level: Level, release: string, key: string): PlanNode {
  return {
    level,
    key,
    reference: work.reference,
    kind: work.kind,
    title: work.title,
    target: work.target,
    release,
    summary: null,
    work,
    children: [],
  }
}

function soleSelf(work: WorkPlan[], reference: string | null): WorkPlan | null {
  const [only] = work
  return reference !== null && work.length === 1 && only !== undefined && only.reference === reference ? only : null
}

function storyNode(story: StoryPlan, release: string): PlanNode {
  const key = `S:${story.reference ?? '-'}`
  const self = soleSelf(story.work, story.reference)
  if (self) {
    return workNode(self, 'story', release, key)
  }
  return {
    level: 'story',
    key,
    reference: story.reference,
    kind: story.reference === null ? null : 'story',
    title: story.title ?? 'No story',
    target: story.target,
    release,
    summary: story,
    work: null,
    children: story.work.map((work) => workNode(work, 'task', release, `T:${work.reference}`)),
  }
}

function epicNode(epic: EpicPlan, release: string): PlanNode {
  const key = `E:${epic.reference ?? '-'}`
  const self = soleSelf(
    epic.stories.flatMap((story) => story.work),
    epic.reference,
  )
  if (self) {
    return workNode(self, 'epic', release, key)
  }
  return {
    level: 'epic',
    key,
    reference: epic.reference,
    kind: epic.reference === null ? null : 'epic',
    title: epic.title ?? 'No epic',
    target: epic.target,
    release,
    summary: epic,
    work: null,
    children: epic.stories.map((story) => storyNode(story, release)),
  }
}

function releaseNode(release: ReleasePlan): PlanNode {
  return {
    level: 'release',
    key: `R:${release.declared_by}:${release.name}`,
    reference: `release ${release.name}`,
    kind: null,
    title: release.name,
    target: release.target,
    release: release.name,
    summary: release,
    work: null,
    children: release.epics.map((epic) => epicNode(epic, release.name)),
  }
}

interface Builder {
  node: PlanNode
  key: string
  summaries: ForecastSummary[]
  contributions: { release: string; segments: TimedSegment[]; hatched: boolean }[]
  children: Map<string, Builder>
}

function mergePieces(segments: { start: number; end: number; done: boolean; hatched: boolean }[]): Piece[] {
  const sorted = [...segments].sort((a, b) => a.start - b.start)
  const merged: Piece[] = []
  for (const segment of sorted) {
    const last = merged[merged.length - 1]
    if (last && segment.start <= last.end + 1e-6 && last.done === segment.done && last.hatched === segment.hatched) {
      last.end = Math.max(last.end, segment.end)
    } else {
      merged.push({ ...segment })
    }
  }
  return merged
}

function fragmentsOf(builder: Builder): Fragment[] {
  const byRelease = new Map<string, Builder['contributions']>()
  for (const contribution of builder.contributions) {
    const list = byRelease.get(contribution.release) ?? []
    list.push(contribution)
    byRelease.set(contribution.release, list)
  }
  return [...byRelease].flatMap(([release, contributions]) => {
    const segments = contributions.flatMap(({ segments, hatched }) => segments.map((segment) => ({ ...segment, hatched })))
    if (segments.length === 0) {
      return []
    }
    const worked = segments
      .filter((segment) => segment.phase !== 'validation')
      .map(({ start, end, phase, hatched }) => ({ start, end, done: phase === 'done', hatched }))
    return [
      {
        release,
        start: Math.min(...segments.map((segment) => segment.start)),
        end: Math.max(...segments.map((segment) => segment.end)),
        pieces: mergePieces(worked),
      },
    ]
  })
}

function missedTargets(roadmap: Roadmap): Map<string, string> {
  const missed = new Map<string, string>()
  for (const warning of roadmap.warnings) {
    if (warning.kind === 'missed_target') {
      for (const item of warning.items) {
        missed.set(item.reference, item.detail ?? 'target missed')
      }
    }
  }
  return missed
}

function workBar(work: WorkPlan, segments: TimedSegment[]): WorkBar {
  return {
    stage: work.stage,
    hours: work.hours,
    testHours: work.test_hours,
    tester: work.tester,
    basis: work.basis,
    hatched: isHatched(work),
    segments,
    start: work.start,
    delivered: work.delivered,
    closed: work.closed,
  }
}

const emptyRelease: ReleasePlan = {
  name: '',
  declared_by: '',
  target: null,
  hours: 0,
  test_hours: 0,
  span: null,
  closed: null,
  done: 0,
  open: 0,
  epics: [],
}

export function buildRows(roadmap: Roadmap, active: readonly Level[]): GanttRow[] {
  const deepest = Math.max(...active.map(levelRank))
  const root: Builder = { node: releaseNode(emptyRelease), key: '', summaries: [], contributions: [], children: new Map() }
  const segmentsOf = new Map<WorkPlan, TimedSegment[]>()

  const visit = (node: PlanNode, parent: Builder, shownAbove: Builder[]) => {
    const shown = node.work ? deepest >= levelRank(node.level) : active.includes(node.level)
    let holder = parent
    if (shown) {
      holder = parent.children.get(node.key) ?? {
        node,
        key: `${parent.key}/${node.key}`,
        summaries: [],
        contributions: [],
        children: new Map(),
      }
      parent.children.set(node.key, holder)
      if (node.summary) {
        holder.summaries.push(node.summary)
      }
    }
    const lineage = shown ? [...shownAbove, holder] : shownAbove
    if (node.work) {
      const segments = workSegments(node.work, roadmap)
      segmentsOf.set(node.work, segments)
      for (const builder of lineage) {
        builder.contributions.push({ release: node.release, segments, hatched: isHatched(node.work) })
      }
    }
    for (const child of node.children) {
      visit(child, holder, lineage)
    }
  }
  for (const release of roadmap.releases) {
    visit(releaseNode(release), root, [])
  }

  const missed = missedTargets(roadmap)
  const rows: GanttRow[] = []
  const flatten = (builder: Builder, depth: number) => {
    const { node } = builder
    const work = node.work
    const total = (pick: (summary: ForecastSummary) => number) => builder.summaries.reduce((sum, summary) => sum + pick(summary), 0)
    const open = work ? work.stage !== 'done' : false
    rows.push({
      key: builder.key,
      level: node.level,
      depth,
      reference: node.reference,
      kind: node.kind,
      id: node.level === 'release' ? null : idOf(node.reference),
      title: node.title,
      hours: work ? (open ? work.hours : 0) : total((summary) => summary.hours),
      testHours: work ? (open ? work.test_hours : 0) : total((summary) => summary.test_hours),
      done: work ? Number(!open) : total((summary) => summary.done),
      open: work ? Number(open) : total((summary) => summary.open),
      target: node.target,
      missed: node.reference === null ? null : (missed.get(node.reference) ?? null),
      fragments: fragmentsOf(builder),
      work: work ? workBar(work, segmentsOf.get(work) ?? []) : null,
    })
    for (const child of builder.children.values()) {
      flatten(child, depth + 1)
    }
  }
  for (const child of root.children.values()) {
    flatten(child, 0)
  }
  return rows
}
