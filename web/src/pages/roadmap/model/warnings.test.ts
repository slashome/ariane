import type { ForecastWarning } from '@/shared/api'
import { sampleRoadmap } from './fixtures'
import { buildRows, type GanttRow } from './rows'
import { filterRows, flaggedBy, warningCounters } from './warnings'

function warning(kind: ForecastWarning['kind'], references: string[]): ForecastWarning {
  return { kind, count: references.length, items: references.map((reference) => ({ reference, detail: null })) }
}

function ids(rows: GanttRow[]): string[] {
  return rows.map((row) => row.id ?? row.title)
}

describe('warningCounters', () => {
  it('counts each kind present, in a fixed order, and leaves out the absent ones', () => {
    const counters = warningCounters({
      warnings: [warning('unsized', ['p#T1', 'p#T2']), warning('missed_target', ['p#T3']), warning('without_story', [])],
    })
    expect(counters.map(({ kind, label, count }) => [kind, label, count])).toEqual([
      ['missed_target', 'Missed target', 1],
      ['unsized', 'Unsized', 2],
    ])
    expect(counters.every((counter) => counter.hint.length > 0)).toBe(true)
  })

  it('has nothing to show without warnings', () => {
    expect(warningCounters({ warnings: [] })).toEqual([])
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
