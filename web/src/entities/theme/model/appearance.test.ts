import { readAppearance, resolveAppearance, setAppearance } from './appearance'

describe('appearance', () => {
  it('follows the system unless light or dark is chosen', () => {
    expect(resolveAppearance('system', true)).toBe('dark')
    expect(resolveAppearance('system', false)).toBe('light')
    expect(resolveAppearance('light', true)).toBe('light')
    expect(resolveAppearance('dark', false)).toBe('dark')
  })

  it('remembers the choice in the browser, the system by default', () => {
    localStorage.removeItem('ariane.appearance')
    expect(readAppearance()).toBe('system')
    setAppearance('dark')
    expect(readAppearance()).toBe('dark')
    localStorage.setItem('ariane.appearance', 'neon')
    expect(readAppearance()).toBe('system')
  })
})
