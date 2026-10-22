import type { Item } from '@/shared/api'
import { blockersOf, childrenOf, dependencyOrder, indexByReference, progressOf } from './progress'
import { groupBy } from './groups'

const item = (id: string, extra: Partial<Item> = {}): Item => ({
  node: 'p',
  id,
  reference: `p#${id}`,
  title: id,
  kind: 'task',
  archived: false,
  status: 'todo',
  category: 'todo',
  size: null,
  impact: null,
  priority: null,
  created: null,
  type: null,
  release: null,
  milestone: null,
  parent: null,
  depends_on: [],
  external: [],
  version: 1,
  path: `tree/p/tasks/${id}.md`,
  ...extra,
})

describe('progressOf', () => {
  it('counts done items against the scope, dropped ones left out', () => {
    const progress = progressOf([
      item('T1', { category: 'done' }),
      item('T2', { category: 'in_progress' }),
      item('T3', { category: 'dropped' }),
      item('T4', { category: 'done' }),
      item('T5', { category: null }),
    ])
    expect(progress.total).toBe(3)
    expect(progress.done).toBe(2)
    expect(progress.ratio).toBeCloseTo(2 / 3)
    expect(progress.counts.dropped).toBe(1)
  })

  it('is zero, not NaN, for a story without tasks', () => {
    expect(progressOf([]).ratio).toBe(0)
  })
})

describe('relations between items', () => {
  const story = item('S1', { kind: 'story' })
  const first = item('T1', { parent: 'p#S1', category: 'done' })
  const second = item('T2', { parent: 'p#S1', depends_on: ['p#T1', 'p#T3', 'q#T9'] })
  const third = item('T3', { depends_on: ['p#T2'] })

  it('finds the children of a story by their parent', () => {
    expect(childrenOf([story, first, second, third], 'p#S1')).toEqual([first, second])
    expect(childrenOf([story], null)).toEqual([])
  })

  it('tells which dependencies still block an item', () => {
    const blockers = blockersOf(second, indexByReference([first, second, third]))
    expect(blockers.open).toEqual([third])
    expect(blockers.missing).toEqual(['q#T9'])
  })

  it('orders items after what they depend on, surviving a cycle', () => {
    const ordered = dependencyOrder([second, first, third]).map((entry) => entry.id)
    expect(ordered.indexOf('T1')).toBeLessThan(ordered.indexOf('T2'))
    expect(ordered).toHaveLength(3)
  })
})

describe('groupBy', () => {
  it('sorts groups naturally and puts the items without a key last', () => {
    const groups = groupBy(
      [item('A', { milestone: '0.10' }), item('B', { milestone: '0.9' }), item('C'), item('D', { milestone: '0.9' })],
      (entry) => entry.milestone,
      'No milestone',
    )
    expect(groups.map((group) => [group.label, group.items.map((entry) => entry.id)])).toEqual([
      ['0.9', ['B', 'D']],
      ['0.10', ['A']],
      ['No milestone', ['C']],
    ])
  })
})
