import { isColor } from '@/entities/theme'
import { presets, sameTheme } from './presets'

describe('theme presets', () => {
  it('are valid colors, Ariane first', () => {
    expect(presets[0]?.theme).toEqual({ primary: '#3760c9', secondary: '#3760c9' })
    expect(presets.every(({ theme }) => isColor(theme.primary) && isColor(theme.secondary))).toBe(true)
  })

  it('compare two themes regardless of case', () => {
    expect(sameTheme({ primary: '#AA0000', secondary: '#00aa00' }, { primary: '#aa0000', secondary: '#00AA00' })).toBe(true)
    expect(sameTheme({ primary: '#aa0000', secondary: '#00aa00' }, undefined)).toBe(false)
  })
})
