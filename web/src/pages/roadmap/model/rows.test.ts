import { at, makeEpic, makeRelease, makeRoadmap, makeStory, makeWork, now, planned, sampleRoadmap } from './fixtures'
import type { Level } from './levels'
import { buildRows, workSegments, type GanttRow } from './rows'

function outline(rows: GanttRow[]): string[] {
  return rows.map((row) => `${'  '.repeat(row.depth)}${row.level} ${row.id ?? row.title}`)
}

function rowsAt(levels: Level[]) {
  return buildRows(sampleRoadmap(), levels)
}

describe('buildRows', () => {
  it('nests every active level under the one above, an unsplit story or epic staying one block', () => {
    expect(outline(rowsAt(['release', 'epic', 'story', 'task']))).toEqual([
      'release 0.1',
      '  epic E1',
      '    story S1',
      '      task T1',
      '      task T2',
      '    story S2',
      '  epic E2',
      '  epic No epic',
      '    story No story',
      '      task T3',
      'release 0.2',
      '  epic E1',
      '    story S3',
      '      task T4',
    ])
  })

  it('flattens skipped levels: with releases and tasks, work sits directly under its release', () => {
    expect(outline(rowsAt(['release', 'task']))).toEqual([
      'release 0.1',
      '  task T1',
      '  task T2',
      '  story S2',
      '  epic E2',
      '  task T3',
      'release 0.2',
      '  task T4',
    ])
  })

  it('folds work into the deepest active level when tasks are off', () => {
    expect(outline(rowsAt(['release']))).toEqual(['release 0.1', 'release 0.2'])
    expect(outline(rowsAt(['story']))).toEqual(['story S1', 'story S2', 'epic E2', 'story No story', 'story S3'])
  })

  it('gives an epic spanning two releases one row and one fragment per release when releases are off', () => {
    const rows = rowsAt(['epic'])
    expect(outline(rows)).toEqual(['epic E1', 'epic E2', 'epic No epic'])
    const epic = rows[0]
    expect(epic?.fragments.map((fragment) => [fragment.release, fragment.end])).toEqual([
      ['0.1', 30],
      ['0.2', 70],
    ])
    expect(epic?.fragments[1]?.start).toBe(60)
    expect(epic?.hours).toBe(32)
    expect(epic?.done).toBe(1)
    expect(epic?.open).toBe(3)
  })

  it('draws worked pieces solid between gaps, done and guessed work kept apart', () => {
    const [epic] = rowsAt(['epic'])
    const pieces = epic?.fragments[0]?.pieces ?? []
    expect(pieces.filter((piece) => !piece.done).map(({ start, end, hatched }) => [start, end, hatched])).toEqual([
      [0, 15, false],
      [15, 30, true],
    ])
    expect(pieces.filter((piece) => piece.done)).toHaveLength(1)
  })

  it('hatches default and provisional sizes, never finished work', () => {
    const rows = rowsAt(['task'])
    const hatched = Object.fromEntries(rows.map((row) => [row.id, row.work?.hatched]))
    expect(hatched).toEqual({ T1: false, T2: false, S2: true, E2: true, T3: false, T4: false })
  })

  it('marks a summary guessed as soon as one of its open parts is guessed, at any level shown', () => {
    const rows = rowsAt(['release', 'epic', 'story', 'task']).filter((row) => row.work === null)
    expect(rows.map((row) => `${outline([row])[0]?.trim()} ${row.guessed}`)).toEqual([
      'release 0.1 true',
      'epic E1 true',
      'story S1 false',
      'epic No epic false',
      'story No story false',
      'release 0.2 false',
      'epic E1 false',
      'story S3 false',
    ])
    expect(rowsAt(['epic']).find((row) => row.id === 'E1')?.guessed).toBe(true)
  })

  it('never counts finished work as guessed', () => {
    const roadmap = makeRoadmap([
      makeRelease('0.1', [makeEpic('p#E1', [makeStory('p#S1', [makeWork('p#T1', { stage: 'done', basis: 'default' }), planned('p#T2', 0, 4, 1)])])]),
    ])
    expect(buildRows(roadmap, ['release']).map((row) => row.guessed)).toEqual([false])
  })

  it('lists the references each row covers, hidden levels included', () => {
    const [release] = rowsAt(['release'])
    expect(release?.covers).toEqual(expect.arrayContaining(['release 0.1', 'p#E1', 'p#S1', 'p#T1', 'p#T2', 'p#S2', 'p#E2', 'p#T3']))
    expect(release?.covers).not.toContain('p#T4')
  })

  it('marks the targets the forecast misses, from its warnings', () => {
    const rows = rowsAt(['task'])
    expect(rows.find((row) => row.id === 'T2')?.missed).toBe('target 2026-11-12, forecast 2026-11-14')
    expect(rows.find((row) => row.id === 'T3')?.missed).toBeNull()
  })

  it('names a release by `release <name>` to find its missed target', () => {
    const roadmap = makeRoadmap([makeRelease('1.0', [], { target: '2026-12-01' })], {
      warnings: [{ kind: 'missed_target', count: 1, items: [{ reference: 'release 1.0', detail: 'late' }] }],
    })
    expect(buildRows(roadmap, ['release'])[0]?.missed).toBe('late')
  })

  it('keeps an empty release as a row without bars', () => {
    const rows = buildRows(makeRoadmap([makeRelease('3.0', [])]), ['release', 'task'])
    expect(rows).toHaveLength(1)
    expect(rows[0]?.fragments).toEqual([])
  })
})

describe('workSegments', () => {
  it('places finished work on the day it closed, before now', () => {
    const work = makeWork('p#T1', { stage: 'done', closed: { date: '2026-11-10', approximate: true } })
    const [segment] = workSegments(work, makeRoadmap([]))
    const midnight = (new Date(2026, 10, 10).getTime() - Date.parse(now)) / 3_600_000
    expect(segment).toEqual({ phase: 'done', start: midnight, end: midnight + 24 })
  })

  it('stops a day closed today at now', () => {
    const work = makeWork('p#T1', { stage: 'done', closed: { date: '2026-11-13', approximate: false } })
    expect(workSegments(work, makeRoadmap([]))[0]?.end).toBeLessThanOrEqual(0)
  })

  it('leaves out finished work without a closing date', () => {
    expect(workSegments(makeWork('p#T1', { stage: 'done' }), makeRoadmap([]))).toEqual([])
  })

  it('adds a validation by others as a trailing segment', () => {
    const work = planned('p#T1', 0, 4, 0, { tester: 'others', validation: { start: at(4), end: at(6) } })
    expect(workSegments(work, makeRoadmap([])).map((segment) => segment.phase)).toEqual(['work', 'test', 'validation'])
  })

  it('reads dated moments from their date, and skips work too far ahead to have one', () => {
    const roadmap = makeRoadmap([], { dated: true })
    const dated = planned('p#T1', 0, 4, 0, {
      segments: [{ phase: 'work', start: { hours: 0, at: '2026-11-14T09:00:00+01:00' }, end: { hours: 4, at: '2026-11-14T13:00:00+01:00' } }],
    })
    expect(workSegments(dated, roadmap)).toEqual([{ phase: 'work', start: 20, end: 24 }])
    const far = planned('p#T2', 0, 4, 0, { segments: [{ phase: 'work', start: at(90_000), end: at(90_004) }] })
    expect(workSegments(far, roadmap)).toEqual([])
  })
})

describe('an epic without stories', () => {
  it('is a single block whatever the levels below it', () => {
    const roadmap = makeRoadmap([makeRelease('0.1', [makeEpic('p#E9', [makeStory(null, [planned('p#E9', 0, 12, 3, { kind: 'epic', basis: 'default' })])])])])
    const rows = buildRows(roadmap, ['release', 'epic', 'story', 'task'])
    expect(outline(rows)).toEqual(['release 0.1', '  epic E9'])
    expect(rows[1]?.work?.hatched).toBe(true)
  })
})
