import { makeItem, type Estimate, type Item, type NodeSummary, type Release } from '@/shared/api'
import { makeEpic, makeRelease, makeRoadmap, makeStory, planned } from './fixtures'
import {
  assignment,
  backlogItems,
  boardList,
  columnItems,
  columnSummary,
  defaultBacklogFilters,
  displayOrder,
  outline,
  planningItems,
  releaseUsers,
  typesOf,
  zoneRelease,
} from './release-board'

const parked: Release = { name: 'plus-tard', parking: true, target: null }
const scheduled: Release = { name: '0.2', parking: false, target: '2027-01-15' }
const releases: Release[] = [parked, scheduled, { name: '0.3', parking: false, target: null }]

const nodes: NodeSummary[] = [
  { path: 'p', items: 0, workflow: [], releases, releases_from: 'p' },
  { path: 'p/c', items: 0, workflow: [], releases, releases_from: 'p' },
  { path: 'p/d', items: 0, workflow: [], releases: [], releases_from: 'p/d' },
]

function item(id: string, overrides: Partial<Item> = {}): Item {
  const kind = overrides.kind ?? (id.startsWith('E') ? 'epic' : id.startsWith('S') ? 'story' : 'task')
  return makeItem({ id, kind, title: `Title ${id}`, path: `tree/p/${id}.md`, ...overrides })
}

const sized: Estimate = { hours: 6, remaining_hours: 6, test_hours: 0, remaining_test_hours: 0, tester: 'owner', basis: 'size', size: 'M', parts: 1, guessed: 0 }

describe('the list a board shows', () => {
  it('names the node that declares it, and whether it is inherited', () => {
    expect(boardList(nodes, 'p')).toEqual({ declaredIn: 'p', inherited: false, releases })
    expect(boardList(nodes, 'p/c').inherited).toBe(true)
    expect(boardList(nodes, 'q')).toEqual({ declaredIn: null, inherited: false, releases: [] })
  })

  it('shows parking releases last, the others in declared order', () => {
    expect(displayOrder(releases).map((release) => release.name)).toEqual(['0.2', '0.3', 'plus-tard'])
  })

  it('keeps the live epics, stories and tasks of the scope that plan against the same list', () => {
    const items = [
      item('T1'),
      item('T2', { node: 'p/c', path: 'tree/p/c/T2.md' }),
      item('T3', { node: 'p/d', path: 'tree/p/d/T3.md' }),
      item('T4', { archived: true }),
      item('R1', { kind: 'reflexion' }),
      item('E1'),
    ]
    expect(planningItems(items, nodes, 'p', 'p').map((entry) => entry.id)).toEqual(['T1', 'T2', 'E1'])
  })
})

describe('the backlog', () => {
  const items = [
    item('T1', { type: 'bug', title: 'Écran figé' }),
    item('T2', { effective_release: { name: '0.2', source: 'story', from: 'p#S1', position: 1, parking: false } }),
    item('T3', { category: 'done' }),
    item('S1', { type: 'code' }),
  ]

  it('shows the open unplanned items, or every open one', () => {
    expect(backlogItems(items, defaultBacklogFilters).map((entry) => entry.id)).toEqual(['T1', 'S1'])
    expect(backlogItems(items, { ...defaultBacklogFilters, show: 'all' }).map((entry) => entry.id)).toEqual(['T1', 'T2', 'S1'])
  })

  it('filters by level, type and text, accents ignored', () => {
    expect(backlogItems(items, { ...defaultBacklogFilters, kinds: ['story'] }).map((entry) => entry.id)).toEqual(['S1'])
    expect(backlogItems(items, { ...defaultBacklogFilters, type: 'bug' }).map((entry) => entry.id)).toEqual(['T1'])
    expect(backlogItems(items, { ...defaultBacklogFilters, text: 'ecran' }).map((entry) => entry.id)).toEqual(['T1'])
    expect(backlogItems(items, { ...defaultBacklogFilters, text: 's1' }).map((entry) => entry.id)).toEqual(['S1'])
    expect(typesOf(items)).toEqual(['bug', 'code'])
  })
})

describe('a release column', () => {
  it('holds the open items whose own release it is', () => {
    const items = [item('T1', { release: '0.2' }), item('T2', { release: '0.2', category: 'done' }), item('T3', { release: '0.3' })]
    expect(columnItems(items, '0.2').map((entry) => entry.id)).toEqual(['T1'])
  })

  it('counts every item of its list that names it, archived ones included', () => {
    const items = [
      item('T1', { release: '0.2', reference: 'p#T1' }),
      item('T2', { release: '0.2', reference: 'p#T2', archived: true }),
      item('T3', { release: '0.2', reference: 'p/d#T3', node: 'p/d' }),
    ]
    expect(releaseUsers(items, nodes, 'p', '0.2')).toEqual(['p#T1', 'p#T2'])
  })

  it('reads its hours, end and counts from the forecast, ~ when a part is guessed', () => {
    const roadmap = makeRoadmap([
      makeRelease('0.2', [makeEpic(null, [makeStory(null, [planned('p#T1', 0, 10, 2, { basis: 'default' })])])], {
        hours: 10,
        test_hours: 2,
        span: { start: { hours: 0, at: '2026-11-13T12:00:00Z' }, end: { hours: 12, at: '2026-11-20T17:00:00Z' } },
        done: 1,
        open: 2,
      }),
    ], { dated: true })
    expect(columnSummary(roadmap, scheduled, [])).toEqual({ hours: '~12 h', end: '2026-11-20', done: 1, open: 2 })
  })

  it('sums the estimates of its cards when the forecast does not schedule it', () => {
    const cards = [item('T1', { estimate: sized }), item('T2', { estimate: sized })]
    expect(columnSummary(makeRoadmap([]), parked, cards)).toEqual({ hours: '12 h', end: null, done: 0, open: 2 })
  })
})

describe('the outline of a list', () => {
  it('nests tasks under their story and stories under their epic, naming a parent that is elsewhere', () => {
    const known = new Map([['p#E1', item('E1', { reference: 'p#E1', title: 'Roadmap' })]])
    const entries = outline(
      [
        item('T2', { parent: 'p#S1', epic: 'p#E1' }),
        item('S1', { reference: 'p#S1', parent: 'p#E1', epic: 'p#E1' }),
        item('T9'),
        item('T3', { parent: 'p#S1', epic: 'p#E1' }),
      ],
      known,
    )
    expect(entries.map((entry) => (entry.type === 'heading' ? `# ${entry.label}` : `${entry.depth} ${entry.item.id}`))).toEqual([
      '# E1 Roadmap',
      '1 S1',
      '2 T2',
      '2 T3',
      '0 T9',
    ])
  })
})

describe('dropping a card', () => {
  const task = item('T1', { release: '0.2', effective_release: { name: '0.2', source: 'own', from: null, position: 1, parking: false } })

  it('plans it in the release of the column, or in none on the backlog', () => {
    expect(zoneRelease('release:0.3')).toBe('0.3')
    expect(assignment(task, 'release:0.3', releases)).toEqual({
      release: '0.3',
      optimistic: { release: '0.3', effective_release: { name: '0.3', source: 'own', from: null, position: 2, parking: false } },
    })
    expect(assignment(task, 'backlog', releases)).toEqual({ release: null, optimistic: { release: null, effective_release: null } })
  })

  it('never plans it in a closed release', () => {
    const closed = releases.map((release) => (release.name === '0.3' ? { ...release, closed: '2026-11-14' } : release))
    expect(assignment(task, 'release:0.3', closed)).toBeNull()
  })

  it('does nothing on its own column or outside any zone', () => {
    expect(assignment(task, 'release:0.2', releases)).toBeNull()
    expect(assignment(item('T2'), 'backlog', releases)).toBeNull()
    expect(assignment(task, 'elsewhere', releases)).toBeNull()
  })
})
