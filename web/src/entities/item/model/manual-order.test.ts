import { makeItem, type Item } from '@/shared/api'
import { compareManualOrder, itemsRankedByMove, placeInOrder, sortByManualOrder } from './manual-order'

const ids = (items: Item[]) => [...items].sort(compareManualOrder).map((item) => item.id)

describe('manual order', () => {
  it('puts ranked items first by ascending rank, negative ranks included', () => {
    expect(ids([makeItem({ id: 'T1', rank: 1500 }), makeItem({ id: 'T2', priority: 'high' }), makeItem({ id: 'T3', rank: -1000 })])).toEqual(['T3', 'T1', 'T2'])
  })

  it('orders unranked items by descending priority, then ids in natural order', () => {
    expect(ids([makeItem({ id: 'T10' }), makeItem({ id: 'T9' }), makeItem({ id: 'T2', priority: 'low' }), makeItem({ id: 'T3', priority: 'high' })])).toEqual(['T3', 'T2', 'T9', 'T10'])
  })

  it('breaks equal ranks by id and ignores the rank of an archived item', () => {
    expect(ids([makeItem({ id: 'T12', rank: 2000 }), makeItem({ id: 'T3', rank: 2000 }), makeItem({ id: 'T1', rank: 1, archived: true })])).toEqual(['T3', 'T12', 'T1'])
  })

  it('keeps nodes and kinds in the order they first appear in, and never compares ranks across nodes', () => {
    const items = [
      makeItem({ node: 'p', id: 'T2', rank: 9000 }),
      makeItem({ node: 'p/a', id: 'T1', rank: -5 }),
      makeItem({ node: 'p', id: 'S1', kind: 'story' }),
      makeItem({ node: 'p', id: 'T1', rank: 1000 }),
    ]
    expect(sortByManualOrder(items, (item) => item).map((item) => `${item.node}#${item.id}`)).toEqual(['p#T1', 'p#T2', 'p#S1', 'p/a#T1'])
  })

  it('moves an item before or after another, or to the top, carrying its changed fields', () => {
    const items = [makeItem({ id: 'T1' }), makeItem({ id: 'T2' }), makeItem({ id: 'T3' })]
    const [first, , third] = items as [Item, Item, Item]
    expect(placeInOrder(items, third, 'before', first).map((item) => item.id)).toEqual(['T3', 'T1', 'T2'])
    expect(placeInOrder(items, first, 'after', third).map((item) => item.id)).toEqual(['T2', 'T3', 'T1'])
    expect(placeInOrder(items, third, 'top', null, { status: 'done' }).map((item) => `${item.id}:${item.status}`)).toEqual(['T3:done', 'T1:todo', 'T2:todo'])
  })

  describe('the items a move ranks', () => {
    const node = [
      makeItem({ id: 'T1', path: 'T1', rank: 1000 }),
      makeItem({ id: 'T2', path: 'T2', rank: 2000 }),
      makeItem({ id: 'T3', path: 'T3', priority: 'high' }),
      makeItem({ id: 'T4', path: 'T4' }),
      makeItem({ id: 'T5', path: 'T5', category: 'done', status: 'done' }),
    ]
    const [t1, t2, t3, t4, t5] = node as [Item, Item, Item, Item, Item]

    it('ranks every unranked item of the node and kind, the moved one included, when it lands below an unranked item', () => {
      expect(itemsRankedByMove(node, t1, 'after', t3)).toBe(4)
      expect(itemsRankedByMove(node, t2, 'before', t5)).toBe(4)
      expect(itemsRankedByMove(node, t5, 'after', t4)).toBe(3)
    })

    it('ranks nothing else when the move lands above every unranked item', () => {
      expect(itemsRankedByMove(node, t4, 'before', t3)).toBe(0)
      expect(itemsRankedByMove(node, t4, 'after', t2)).toBe(0)
      expect(itemsRankedByMove(node, t2, 'before', t1)).toBe(0)
      expect(itemsRankedByMove(node, t5, 'top', null)).toBe(0)
      expect(itemsRankedByMove(node, t5, 'bottom', null)).toBe(0)
    })

    it('leaves out archived items, items without id, and other nodes and kinds', () => {
      const crowded = [
        ...node,
        makeItem({ id: 'T6', path: 'T6', archived: true }),
        makeItem({ id: null, path: 'loose' }),
        makeItem({ id: 'T7', path: 'T7', node: 'q' }),
        makeItem({ id: 'S1', path: 'S1', kind: 'story' }),
      ]
      expect(itemsRankedByMove(crowded, t1, 'after', t4)).toBe(4)
    })
  })
})
