import type { ForecastWarning } from '@/shared/api'
import { makeEpic, makeRelease, makeRoadmap, makeStory, makeWork, sampleRoadmap } from './fixtures'
import { buildRows, type GanttRow } from './rows'
import { filterRows, flaggedBy, mutedHint, scheduledWork, warningCounters, type WarningCounter } from './warnings'

function warning(kind: ForecastWarning['kind'], references: string[]): ForecastWarning {
  return { kind, count: references.length, items: references.map((reference) => ({ reference, detail: null })) }
}

function ids(rows: GanttRow[]): string[] {
  return rows.map((row) => row.id ?? row.title)
}

describe('warningCounters', () => {
  it('counts each kind present, in a fixed order, and leaves out the absent ones', () => {
    const counters = warningCounters({
      releases: [],
      warnings: [warning('unsized', ['p#T1', 'p#T2']), warning('missed_target', ['p#T3']), warning('without_story', [])],
    })
    expect(counters.map(({ kind, label, count }) => [kind, label, count])).toEqual([
      ['missed_target', 'Missed target', 1],
      ['unsized', 'Unsized', 2],
    ])
    expect(counters.every((counter) => counter.hint.length > 0)).toBe(true)
  })

  it('has nothing to show without warnings', () => {
    expect(warningCounters({ releases: [], warnings: [] })).toEqual([])
  })
})

describe('muting a warning that touches most of the scheduled work', () => {
  const scheduled = makeRoadmap([
    makeRelease('0.1', [makeEpic('p#E1', [makeStory('p#S1', [makeWork('p#T1'), makeWork('p#T2')]), makeStory('p#S2', [makeWork('p#T3'), makeWork('p#T4')])])]),
  ])

  function counter(warnings: ForecastWarning[], releases = scheduled.releases): WarningCounter {
    const [only] = warningCounters({ releases, warnings })
    if (only === undefined) {
      throw new Error('no counter')
    }
    return only
  }

  it('counts the work items of the Gantt rows as the scheduled work', () => {
    expect([...scheduledWork(scheduled)]).toEqual(['p#T1', 'p#T2', 'p#T3', 'p#T4'])
  })

  it('mutes a counter flagging more than half of the scheduled work, and says why', () => {
    const unsized = counter([warning('unsized', ['p#T1', 'p#T2', 'p#T3'])])
    expect(unsized.muted).toBe(true)
    expect(unsized.hint).toContain(mutedHint)
  })

  it('keeps a counter flagging half of the scheduled work or less', () => {
    const unsized = counter([warning('unsized', ['p#T1', 'p#T2'])])
    expect(unsized.muted).toBe(false)
    expect(unsized.hint).not.toContain(mutedHint)
  })

  it('weighs only the flagged items that are scheduled work', () => {
    expect(counter([warning('missed_target', ['release 0.1', 'p#E1', 'p#S1', 'p#S2'])]).muted).toBe(false)
  })

  it('mutes nothing without scheduled work', () => {
    expect(counter([warning('without_release', ['p#T9'])], []).muted).toBe(false)
  })
})

describe('filtering by a warning', () => {
  const roadmap = { ...sampleRoadmap(), warnings: [warning('provisional', ['p#S2']), warning('missed_target', ['release 0.2'])] }

  it('keeps the flagged rows and their ancestors for context', () => {
    const rows = buildRows(roadmap, ['release', 'epic', 'story', 'task'])
    expect(ids(filterRows(rows, flaggedBy(roadmap, 'provisional')))).toEqual(['0.1', 'E1', 'S2'])
  })

  it('keeps the row that holds a flagged item a hidden level would show', () => {
    const rows = buildRows(roadmap, ['epic'])
    expect(ids(filterRows(rows, flaggedBy(roadmap, 'provisional')))).toEqual(['E1'])
  })

  it('keeps a flagged release alone', () => {
    const rows = buildRows(roadmap, ['release', 'epic'])
    expect(ids(filterRows(rows, flaggedBy(roadmap, 'missed_target')))).toEqual(['0.2'])
  })

  it('keeps every row without a filter', () => {
    const rows = buildRows(roadmap, ['release'])
    expect(flaggedBy(roadmap, null)).toBeNull()
    expect(filterRows(rows, null)).toBe(rows)
  })
})
