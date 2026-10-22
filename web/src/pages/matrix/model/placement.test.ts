import type { Item, Schema } from '@/shared/api'
import { cellId, isQuickWin, parseCellId, placeItems } from './placement'

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
    created: null,
    external: [],
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
})
