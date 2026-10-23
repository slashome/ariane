import { isApple, isPaletteShortcut } from './shortcut'

const press = (key: string, held: Partial<Record<'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey', boolean>> = {}) => ({
  key,
  metaKey: false,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  ...held,
})

describe('isPaletteShortcut', () => {
  it('is ⌘K on an Apple system and Ctrl+K elsewhere', () => {
    expect(isPaletteShortcut(press('k', { metaKey: true }), true)).toBe(true)
    expect(isPaletteShortcut(press('K', { metaKey: true }), true)).toBe(true)
    expect(isPaletteShortcut(press('k', { ctrlKey: true }), true)).toBe(false)
    expect(isPaletteShortcut(press('k', { ctrlKey: true }), false)).toBe(true)
    expect(isPaletteShortcut(press('k', { metaKey: true }), false)).toBe(false)
  })

  it('ignores another key or another modifier held', () => {
    expect(isPaletteShortcut(press('j', { metaKey: true }), true)).toBe(false)
    expect(isPaletteShortcut(press('k', { metaKey: true, shiftKey: true }), true)).toBe(false)
    expect(isPaletteShortcut(press('k'), false)).toBe(false)
  })
})

describe('isApple', () => {
  it('recognizes macOS and iOS platforms', () => {
    expect(isApple('MacIntel')).toBe(true)
    expect(isApple('macOS')).toBe(true)
    expect(isApple('iPhone')).toBe(true)
    expect(isApple('Win32')).toBe(false)
    expect(isApple('Linux x86_64')).toBe(false)
  })
})
