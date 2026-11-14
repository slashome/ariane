import type { Unscheduled } from '@/shared/api'
import { buildColumns, narrowColumns, parseUnfolded, reasonLabel, toggleColumn, unplannedKey } from './columns'

function unscheduled(reference: string, reason: Unscheduled['reason'], release: string | null = null): Unscheduled {
  return { reference, node: 'p', kind: 'task', title: `Work ${reference}`, reason, release }
}

const roadmap = {
  unscheduled: [
    unscheduled('p#T1', 'parking', 'plus-tard'),
    unscheduled('p#T2', 'no_release'),
    unscheduled('p#T3', 'parking', 'someday'),
    unscheduled('p#T4', 'parking', 'plus-tard'),
    unscheduled('p#T5', 'undeclared_release', '9.9'),
    unscheduled('p#T6', 'cycle'),
  ],
}

describe('buildColumns', () => {
  it('gives each parking release its column, by name, then one Unplanned column for every other reason', () => {
    const columns = buildColumns(roadmap)
    expect(columns.map((column) => [column.key, column.name, column.entries.map((entry) => entry.id)])).toEqual([
      ['parking:plus-tard', 'plus-tard', ['T1', 'T4']],
      ['parking:someday', 'someday', ['T3']],
      [unplannedKey, 'Unplanned', ['T2', 'T5', 'T6']],
    ])
  })

  it('shows no Unplanned column when everything unscheduled is parked', () => {
    expect(buildColumns({ unscheduled: [unscheduled('p#T1', 'parking', 'plus-tard')] }).map((column) => column.key)).toEqual(['parking:plus-tard'])
    expect(buildColumns({ unscheduled: [] })).toEqual([])
  })

  it('narrows columns to the flagged items and drops the ones left empty', () => {
    const narrowed = narrowColumns(buildColumns(roadmap), new Set(['p#T2', 'p#T4']))
    expect(narrowed.map((column) => [column.name, column.entries.length])).toEqual([
      ['plus-tard', 1],
      ['Unplanned', 1],
    ])
    expect(narrowColumns(buildColumns(roadmap), null)).toHaveLength(3)
  })
})

describe('reasonLabel', () => {
  it('says why work is not scheduled', () => {
    expect(reasonLabel({ reason: 'no_release', release: null })).toBe('no release')
    expect(reasonLabel({ reason: 'undeclared_release', release: '9.9' })).toBe('release 9.9 not declared')
    expect(reasonLabel({ reason: 'cycle', release: null })).toBe('dependency cycle')
    expect(reasonLabel({ reason: 'closed_release', release: '0.1' })).toBe('left in 0.1, closed')
    expect(reasonLabel({ reason: 'blocked_by_cycle', release: null })).toBe('waits on a dependency cycle')
  })
})

describe('folding', () => {
  it('unfolds a column on a first click and folds it back on a second', () => {
    expect(toggleColumn([], 'unplanned')).toEqual(['unplanned'])
    expect(toggleColumn(['parking:plus-tard', 'unplanned'], 'unplanned')).toEqual(['parking:plus-tard'])
  })

  it('reads a stored choice, folding everything when it is unreadable', () => {
    expect(parseUnfolded(['unplanned', 3, null])).toEqual(['unplanned'])
    expect(parseUnfolded('unplanned')).toEqual([])
    expect(parseUnfolded(null)).toEqual([])
  })
})
