import { isHandle, isPersonaId, suggestHandle } from './handle'

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

describe('isPersonaId', () => {
  it('takes a group and a role', () => {
    expect(isPersonaId('user:rh')).toBe(true)
    expect(isPersonaId('rh')).toBe(false)
    expect(isPersonaId('User:rh')).toBe(false)
    expect(isPersonaId('user:')).toBe(false)
  })
})
