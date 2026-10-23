import { isHandle, suggestHandle } from './handle'

describe('handles', () => {
  it('are suggested from the first word of a name', () => {
    expect(suggestHandle('Keyvan Coen')).toBe('keyvan')
    expect(suggestHandle('  Élodie  Martin')).toBe('elodie')
    expect(suggestHandle("1st O'Brien")).toBe('st')
    expect(suggestHandle('—')).toBeNull()
  })

  it('are lower case letters, digits and dashes, starting with a letter', () => {
    expect(isHandle('kcoen-2')).toBe(true)
    expect(isHandle('Keyvan')).toBe(false)
    expect(isHandle('2k')).toBe(false)
  })
})
