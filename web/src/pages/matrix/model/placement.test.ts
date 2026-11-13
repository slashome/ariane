import type { Item, NodeSummary, Pricing, Schema } from '@/shared/api'
import { cellId, isQuickWin, parseCellId, placeItems, sharedPricing, sizeHours } from './placement'

const schema = {
  keys: [
    { name: 'size', type: 'enum', values: ['XS', 'S', 'M', 'L', 'XL'] },
    { name: 'impact', type: 'enum', values: ['low', 'medium', 'high'] },
  ],
} as unknown as Schema

function item(overrides: Partial<Item>): Item {
  return {
    node: 'game',
    id: 'T001',
    title: 'Forge',
    kind: 'task',
    archived: false,
    status: 'todo',
    category: 'todo',
    size: 'XS',
    impact: 'high',
    priority: null,
    rank: null,
    created: null,
    closed: null,
    reference: null,
    type: null,
    personas: [],
    release: null,
    effective_release: null,
    estimate: null,
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
  item({ id: 'T002', size: null, path: 'b' }),
  item({ id: 'T003', impact: 'huge', path: 'c' }),
  item({ id: 'T004', category: 'done', status: 'done', size: 'L', impact: 'low', path: 'd' }),
  item({ id: 'R001', kind: 'reflexion', path: 'e' }),
]

describe('placeItems', () => {
  it('orders impact from high to low and sizes from XS to XL', () => {
    const matrix = placeItems(items, schema, false)
    expect(matrix.impacts).toEqual(['high', 'medium', 'low'])
    expect(matrix.sizes).toEqual(['XS', 'S', 'M', 'L', 'XL'])
  })

  it('puts estimated items in their cell and the rest in the unestimated tray', () => {
    const matrix = placeItems(items, schema, false)
    expect(matrix.cells.get(cellId('high', 'XS'))?.map((entry) => entry.id)).toEqual(['T001'])
    expect(matrix.unestimated.map((entry) => entry.id)).toEqual(['T002', 'T003'])
    expect(matrix.cells.get(cellId('low', 'L'))).toBeUndefined()
  })

  it('shows closed items only when asked', () => {
    const matrix = placeItems(items, schema, true)
    expect(matrix.cells.get(cellId('low', 'L'))?.map((entry) => entry.id)).toEqual(['T004'])
  })

  it('marks small sizes with high impact as quick wins and round-trips cell ids', () => {
    const matrix = placeItems(items, schema, false)
    expect(isQuickWin(matrix, 'high', 'S')).toBe(true)
    expect(isQuickWin(matrix, 'high', 'M')).toBe(false)
    expect(isQuickWin(matrix, 'medium', 'XS')).toBe(false)
    expect(parseCellId(cellId('medium', 'XL'))).toEqual({ impact: 'medium', size: 'XL' })
    expect(parseCellId('tray')).toBeNull()
  })

  it('prices the columns only when every node shown prices sizes alike', () => {
    const pricing: Pricing = { XS: 2, S: 4, M: 12, L: 24, XL: 48, default: 'M', test_ratio: 0.25, tester: 'owner' }
    const node = (path: string, estimates?: Pricing): NodeSummary => ({ path, items: 1, workflow: [], estimates })
    const nodes = [node('game', pricing), node('site', pricing), node('other', { ...pricing, M: 8 }), node('bare')]
    const shared = sharedPricing(nodes, [item({ node: 'game' }), item({ node: 'site' })])
    expect(sizeHours(shared, 'M')).toBe(12)
    expect(sharedPricing(nodes, [item({ node: 'game' }), item({ node: 'other' })])).toBeNull()
    expect(sharedPricing(nodes, [item({ node: 'bare' })])).toBeNull()
    expect(sizeHours(null, 'M')).toBeNull()
  })
})
