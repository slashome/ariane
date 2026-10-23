import { fold, foldText } from './fold'

describe('fold', () => {
  it('ignores case and accents, and spells out ligatures', () => {
    expect(foldText('Été à Noël')).toBe('ete a noel')
    expect(foldText('Œuvre STRAßE Øre')).toBe('oeuvre strasse ore')
    expect(foldText('été')).toBe('ete')
    expect(foldText('日本 Ελλάδα')).toBe('日本 ελλαδα')
  })

  it('points every folded unit back to its code point in the original', () => {
    expect(fold('cœur').origin).toEqual([0, 1, 1, 2, 3])
    expect(fold('ét').origin).toEqual([0, 2])
    expect(fold('😀é').origin).toEqual([0, 0, 1])
  })
})
