import { faviconHref, faviconSvg } from '../lib/favicon'
import { applyTheme } from '../lib/apply-theme'
import { contrast, isColor, readableOnLight } from './color'

describe('theme colors', () => {
  it('accepts only #rrggbb', () => {
    expect(isColor('#3760c9')).toBe(true)
    expect(isColor('#3760C9')).toBe(true)
    expect(isColor('3760c9')).toBe(false)
    expect(isColor('#376')).toBe(false)
  })

  it('measures the contrast of a primary against the light background', () => {
    expect(contrast('#000000', '#ffffff')).toBeCloseTo(21)
    expect(readableOnLight('#3760c9')).toBe(true)
    expect(readableOnLight('#f5e663')).toBe(false)
  })
})

describe('applyTheme', () => {
  it('sets the theme variables and a favicon drawn in the primary', () => {
    applyTheme({ primary: '#aa0000', secondary: '#00aa00' })
    expect(document.documentElement.style.getPropertyValue('--theme-primary')).toBe('#aa0000')
    expect(document.documentElement.style.getPropertyValue('--theme-secondary')).toBe('#00aa00')
    const icon = document.querySelector<HTMLLinkElement>('link[rel="icon"]')
    expect(icon?.getAttribute('href')).toBe(faviconHref('#aa0000'))
    expect(faviconSvg('#aa0000')).toContain('stroke="#aa0000"')
    applyTheme({ primary: '#0000aa', secondary: '#00aa00' })
    expect(document.querySelectorAll('link[rel="icon"]')).toHaveLength(1)
  })
})
