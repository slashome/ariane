import { makeItem } from '@/shared/api'
import { nodeCounts } from './node-counts'

describe('nodeCounts', () => {
  const items = [
    makeItem({ node: 'cmc', id: 'T1' }),
    makeItem({ node: 'cmc/cmc-lib', id: 'T2' }),
    makeItem({ node: 'cmc/cmc-lib', id: 'T3', archived: true }),
  ]
  const paths = ['', 'cmc', 'cmc/cmc-lib']

  it('counts a node alone, or with its sub-nodes, following the toggles', () => {
    const count = (includeArchived: boolean, includeDescendants: boolean) =>
      Object.fromEntries(nodeCounts(items, paths, { includeArchived, includeDescendants }))
    expect(count(false, false)).toEqual({ '': 0, cmc: 1, 'cmc/cmc-lib': 1 })
    expect(count(false, true)).toEqual({ '': 2, cmc: 2, 'cmc/cmc-lib': 1 })
    expect(count(true, true)).toEqual({ '': 3, cmc: 3, 'cmc/cmc-lib': 2 })
  })
})
