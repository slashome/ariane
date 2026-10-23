import { itemPath, referenceFromSplat } from './routes'

describe('item URLs', () => {
  it('nest the node path under the section of the kind', () => {
    expect(itemPath('story', 'cmc/cmc-lib', 'S001')).toBe('/stories/cmc/cmc-lib/S001')
    expect(itemPath('unknown', 'p', 'T1')).toBe('/tasks/p/T1')
  })

  it('round-trip to a reference, the home node included', () => {
    expect(referenceFromSplat('cmc/cmc-lib/S001')).toBe('cmc/cmc-lib#S001')
    expect(itemPath('reflexion', '', 'R1')).toBe('/reflexions/~/R1')
    expect(referenceFromSplat('~/R1')).toBe('#R1')
    expect(referenceFromSplat('S001')).toBeNull()
    expect(referenceFromSplat(undefined)).toBeNull()
  })
})

describe('scopeSearch', () => {
  it('keeps only the node scope, the archive toggle and the search field', async () => {
    const { scopeSearch } = await import('./search-params')
    expect(scopeSearch('?node=p&q=fence&archived=1&category=done')).toBe('?node=p&archived=1&q=fence')
    expect(scopeSearch('?q=fence')).toBe('?q=fence')
    expect(scopeSearch('?category=done')).toBe('')
  })
})
