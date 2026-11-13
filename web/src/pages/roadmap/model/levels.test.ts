import { parseLevels, toggleLevel } from './levels'

describe('toggleLevel', () => {
  it('turns a level off, and back on in its place in R / E / S / T', () => {
    expect(toggleLevel(['release', 'epic', 'story', 'task'], 'epic')).toEqual(['release', 'story', 'task'])
    expect(toggleLevel(['release', 'task'], 'story')).toEqual(['release', 'story', 'task'])
  })

  it('never turns off the last active level', () => {
    expect(toggleLevel(['epic'], 'epic')).toEqual(['epic'])
  })
})

describe('parseLevels', () => {
  it('keeps the known levels of what was stored, in order', () => {
    expect(parseLevels(['task', 'release', 'bogus'])).toEqual(['release', 'task'])
  })

  it('shows every level when nothing usable was stored', () => {
    expect(parseLevels(null)).toEqual(['release', 'epic', 'story', 'task'])
    expect(parseLevels([])).toEqual(['release', 'epic', 'story', 'task'])
    expect(parseLevels('task')).toEqual(['release', 'epic', 'story', 'task'])
  })
})
