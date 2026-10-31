import { makeItem, type Item } from '@/shared/api'
import { compareManualOrder } from './manual-order'

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
})
