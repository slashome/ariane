import type { Item } from '@/shared/api'
import { emptyFilters, filterItems, scopeItems } from './filter-items'

function item(overrides: Partial<Item>): Item {
  return {
    node: 'acme/web',
    id: 'T001',
    title: 'Ship the landing page',
    kind: 'task',
    archived: false,
    status: 'todo',
    category: 'todo',
    size: null,
    impact: null,
    priority: null,
    created: null,
    reference: null,
    type: null,
    release: null,
    milestone: null,
    parent: null,
    depends_on: [],
    external: [],
    branches: [],
    pull_requests: [],
    version: 1,
    path: 'tree/acme/web/tasks/T001-ship.md',
    ...overrides,
  }
}

const items = [
  item({}),
  item({ id: 'T002', title: 'Old cleanup', archived: true, category: 'done', status: 'done', path: 'tree/acme/web/archives/tasks/T002.md' }),
  item({ node: 'acme/web-legacy', id: 'T003', title: 'Legacy port', category: 'blocked', status: 'blocked', path: 'tree/acme/web-legacy/tasks/T003.md' }),
  item({ node: 'acme', id: 'R001', kind: 'reflexion', title: 'Naming', category: null, status: 'open', path: 'tree/acme/reflexions/R001.md' }),
]

const ids = (result: Item[]) => result.map((entry) => entry.id)

describe('filterItems', () => {
  it('hides archived items unless asked', () => {
    expect(ids(filterItems(items, emptyFilters))).toEqual(['T001', 'T003', 'R001'])
    expect(ids(filterItems(items, { ...emptyFilters, includeArchived: true }))).toHaveLength(4)
  })

  it('keeps a node and, when asked, its descendants, never a sibling sharing a prefix', () => {
    const withSubNodes = { ...emptyFilters, includeDescendants: true }
    expect(ids(filterItems(items, { ...withSubNodes, node: 'acme/web' }))).toEqual(['T001'])
    expect(ids(filterItems(items, { ...withSubNodes, node: 'acme' }))).toEqual(['T001', 'T003', 'R001'])
    expect(ids(filterItems(items, { ...emptyFilters, node: 'acme' }))).toEqual(['R001'])
  })

  it('treats the home node with its sub-nodes as the whole tree, alone as its own items', () => {
    expect(ids(filterItems(items, { ...emptyFilters, node: '', includeDescendants: true }))).toEqual(['T001', 'T003', 'R001'])
    expect(ids(filterItems(items, { ...emptyFilters, node: '' }))).toEqual([])
  })

  it('filters by kind and by status category', () => {
    expect(ids(filterItems(items, { ...emptyFilters, kinds: ['reflexion'] }))).toEqual(['R001'])
    expect(ids(filterItems(items, { ...emptyFilters, categories: ['blocked', 'todo'] }))).toEqual(['T001', 'T003'])
  })

  it('searches title, id, node and path case-insensitively', () => {
    expect(ids(filterItems(items, { ...emptyFilters, query: 'LEGACY' }))).toEqual(['T003'])
    expect(ids(filterItems(items, { ...emptyFilters, query: 'r001' }))).toEqual(['R001'])
    expect(ids(filterItems(items, { ...emptyFilters, query: 'tasks/T001' }))).toEqual(['T001'])
  })
})

describe('scopeItems', () => {
  it('applies the archive toggle and the node scope of every view, and the kinds asked', () => {
    const scope = { node: null, includeArchived: false, includeDescendants: false }
    expect(ids(scopeItems(items, scope))).toEqual(['T001', 'T003', 'R001'])
    expect(ids(scopeItems(items, { ...scope, includeArchived: true }))).toEqual(['T001', 'T002', 'T003', 'R001'])
    expect(ids(scopeItems(items, { ...scope, node: 'acme/web', includeArchived: true }, ['task']))).toEqual(['T001', 'T002'])
    expect(ids(scopeItems(items, { ...scope, node: 'acme' }))).toEqual(['R001'])
    expect(ids(scopeItems(items, { ...scope, node: 'acme', includeDescendants: true }, ['task']))).toEqual(['T001', 'T003'])
  })

  it('searches the body when the item carries one', () => {
    const withBody = [item({ body: 'Read https://example.com first' }), item({ id: 'T009' })]
    expect(ids(filterItems(withBody, { ...emptyFilters, query: 'example.com' }))).toEqual(['T001'])
  })
})
