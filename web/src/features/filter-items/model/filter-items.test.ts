import type { Item } from '@/shared/api'
import { emptyFilters, filterItems } from './filter-items'

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

  it('keeps a node and its descendants, not a sibling sharing a prefix', () => {
    expect(ids(filterItems(items, { ...emptyFilters, node: 'acme/web' }))).toEqual(['T001'])
    expect(ids(filterItems(items, { ...emptyFilters, node: 'acme' }))).toEqual(['T001', 'T003', 'R001'])
  })

  it('treats the home node as the whole tree', () => {
    expect(ids(filterItems(items, { ...emptyFilters, node: '' }))).toEqual(['T001', 'T003', 'R001'])
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
