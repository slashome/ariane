import { customEntries, refsOf } from './header-keys'

describe('header keys', () => {
  it('keeps the custom keys, lists joined', () => {
    expect(customEntries({ ariane: 1, id: 'T1', mod: 'Takedowns', tests: ['a', 'b'] })).toEqual([
      ['mod', 'Takedowns'],
      ['tests', 'a, b'],
    ])
    expect(customEntries(null)).toEqual([])
  })

  it('reads refs as strings', () => {
    expect(refsOf({ refs: ['ADR-0008', 12] })).toEqual(['ADR-0008', '12'])
    expect(refsOf({ refs: 'x' })).toEqual([])
  })
})
