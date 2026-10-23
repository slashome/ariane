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
    waiting_for: [],
    waiting_on: null,
    waiting_since: null,
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

describe('filterItems with a typed query', () => {
  const orders = { size: ['XS', 'S', 'M', 'L', 'XL'], impact: ['low', 'medium', 'high'], priority: ['low', 'medium', 'high'] }
  const typed = [
    item({ id: 'T1', title: 'Préparer l’été', status: 'a-faire', category: 'todo', size: 'S', impact: 'high', waiting_for: ['keyvan'] }),
    item({ id: 'T2', title: 'Ranger', status: 'fait', category: 'done', size: 'L', priority: 'low' }),
    item({ id: 'T3', title: 'Archiver', status: 'a-faire', category: 'todo', size: 'M', archived: true }),
    item({ id: 'T4', title: 'Sans taille', status: 'en-cours', category: 'in_progress' }),
  ]
  const found = (query: string) => ids(filterItems(typed, { ...emptyFilters, includeArchived: true, orders, query }))

  it('filters by status, category and open or closed, alternatives with a comma', () => {
    expect(found('status:a-faire')).toEqual(['T1', 'T3'])
    expect(found('status:FAIT,en-cours')).toEqual(['T2', 'T4'])
    expect(found('category:in_progress')).toEqual(['T4'])
    expect(found('is:open')).toEqual(['T1', 'T4'])
    expect(found('is:closed')).toEqual(['T2', 'T3'])
  })

  it('compares sizes, impacts and priorities along the schema order; an unestimated item never matches', () => {
    expect(found('size:<=M')).toEqual(['T1', 'T3'])
    expect(found('size:>S')).toEqual(['T2', 'T3'])
    expect(found('size:m')).toEqual(['T3'])
    expect(found('impact:>=medium')).toEqual(['T1'])
    expect(found('size:<=XXL')).toEqual([])
  })

  it('keeps the items waiting for someone, negates, and mixes with free words, accents ignored', () => {
    expect(found('@Keyvan')).toEqual(['T1'])
    expect(found('-@keyvan is:open')).toEqual(['T4'])
    expect(found('ete status:a-faire')).toEqual(['T1'])
    expect(found('-status:a-faire -ranger')).toEqual(['T4'])
  })

  it('applies nothing for a clause being typed, and searches an unknown key as text', () => {
    expect(found('status: -@ size:<=')).toEqual(['T1', 'T2', 'T3', 'T4'])
    expect(found('tasks:x')).toEqual([])
    expect(found('tasks/T001')).toEqual(['T1', 'T2', 'T3', 'T4'])
  })
})
