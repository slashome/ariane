import type { Item, NodeSummary } from '@/shared/api'
import { buildColumns, targetStatus } from './columns'

const nodes: NodeSummary[] = [
  {
    path: 'game',
    items: 3,
    workflow: [
      { id: 'a-faire', label: 'À faire', category: 'todo' },
      { id: 'a-tester', label: 'À tester', category: 'in_progress' },
      { id: 'valide', label: 'Validé', category: 'done' },
    ],
  },
  {
    path: 'site',
    items: 1,
    workflow: [
      { id: 'todo', label: 'To do', category: 'todo' },
      { id: 'done', label: 'Done', category: 'done' },
    ],
  },
]

function item(overrides: Partial<Item>): Item {
  return {
    node: 'game',
    id: 'T001',
    title: 'Forge',
    kind: 'task',
    archived: false,
    status: 'a-faire',
    category: 'todo',
    size: null,
    impact: null,
    priority: null,
    rank: null,
    created: null,
    reference: null,
    type: null,
    personas: [],
    release: null,
    milestone: null,
    parent: null,
    epic: null,
    depends_on: [],
    external: [],
    branches: [],
    pull_requests: [],
    waiting_for: [],
    waiting_on: null,
    waiting_since: null,
    version: 1,
    path: 'tree/game/tasks/T001.md',
    ...overrides,
  }
}

const items = [
  item({}),
  item({ id: 'T002', status: 'a-tester', category: 'in_progress', path: 'tree/game/tasks/T002.md' }),
  item({ id: 'T003', status: 'valide', category: 'done', archived: true, path: 'tree/game/archives/tasks/T003.md' }),
  item({ id: 'R001', kind: 'reflexion', status: 'open', category: null, path: 'tree/game/reflexions/R001.md' }),
  item({ node: 'site', id: 'T009', status: 'todo', category: 'todo', path: 'tree/site/tasks/T009.md' }),
]

const ids = (column: { items: Item[] }) => column.items.map((entry) => entry.id)

function at<T>(list: T[], index: number): T {
  const value = list[index]
  if (value === undefined) {
    throw new Error(`no element at ${index}`)
  }
  return value
}

describe('buildColumns', () => {
  it('uses the selected node workflow, in order, with its labels', () => {
    const columns = buildColumns(items, nodes, 'game', undefined)
    expect(columns.map((column) => column.label)).toEqual(['À faire', 'À tester', 'Validé'])
    expect(columns.map(ids)).toEqual([['T001', 'T009'], ['T002'], []])
  })

  it('uses one column per category across all nodes, without archives nor reflexions', () => {
    const columns = buildColumns(items, nodes, null, undefined)
    expect(columns.map((column) => column.id)).toEqual([
      'category:backlog',
      'category:todo',
      'category:in_progress',
      'category:waiting',
      'category:blocked',
      'category:done',
      'category:dropped',
    ])
    expect(ids(at(columns, 1))).toEqual(['T001', 'T009'])
    expect(ids(at(columns, 2))).toEqual(['T002'])
    expect(ids(at(columns, 5))).toEqual([])
  })
})

describe('targetStatus', () => {
  const byCategory = buildColumns(items, nodes, null, undefined)

  it('maps a category column to the first status of that category in the item own workflow', () => {
    expect(targetStatus(at(items, 0), at(byCategory, 2), nodes)).toEqual({ ok: true, status: 'a-tester', category: 'in_progress' })
    expect(targetStatus(at(items, 4), at(byCategory, 5), nodes)).toEqual({ ok: true, status: 'done', category: 'done' })
  })

  it('refuses a category the item workflow does not have', () => {
    const result = targetStatus(at(items, 4), at(byCategory, 2), nodes)
    expect(result.ok).toBe(false)
  })

  it('keeps the exact status of a workflow column when the item workflow has it', () => {
    const byStatus = buildColumns(items, nodes, 'game', undefined)
    expect(targetStatus(at(items, 0), at(byStatus, 2), nodes)).toEqual({ ok: true, status: 'valide', category: 'done' })
  })
})
