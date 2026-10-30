import { nextSort, parseSort, sortRows } from './sort'

describe('sortRows', () => {
  const value = (row: { value: string | number | null }) => row.value

  it('orders text naturally, numbers by value, ties in their first order', () => {
    const rows = [{ value: 'T10' }, { value: 't2' }, { value: 'T2' }, { value: 'T1' }]
    expect(sortRows(rows, value, 'asc')).toEqual([{ value: 'T1' }, { value: 't2' }, { value: 'T2' }, { value: 'T10' }])
    expect(sortRows([{ value: 10 }, { value: 9 }], value, 'asc')).toEqual([{ value: 9 }, { value: 10 }])
  })

  it('keeps blank values last in both directions', () => {
    const rows = [{ value: null }, { value: 2 }, { value: '' }, { value: 1 }]
    expect(sortRows(rows, value, 'asc').map(value)).toEqual([1, 2, null, ''])
    expect(sortRows(rows, value, 'desc').map(value)).toEqual([2, 1, null, ''])
  })
})

describe('nextSort', () => {
  it('goes ascending, then descending, then back to no sort', () => {
    const ascending = nextSort(null, 'size')
    expect(ascending).toEqual({ column: 'size', direction: 'asc' })
    const descending = nextSort(ascending, 'size')
    expect(descending).toEqual({ column: 'size', direction: 'desc' })
    expect(nextSort(descending, 'size')).toBeNull()
  })

  it('starts ascending on another column', () => {
    expect(nextSort({ column: 'size', direction: 'desc' }, 'id')).toEqual({ column: 'id', direction: 'asc' })
  })
})

describe('parseSort', () => {
  it('reads a stored sort and forgets a corrupt one', () => {
    expect(parseSort('{"column":"id","direction":"desc"}')).toEqual({ column: 'id', direction: 'desc' })
    expect(parseSort('{"column":"id","direction":"up"}')).toBeNull()
    expect(parseSort('not json')).toBeNull()
    expect(parseSort(null)).toBeNull()
  })
})
