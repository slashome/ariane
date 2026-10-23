import { highlightRanges, splitByRanges } from './highlight'

describe('highlightRanges', () => {
  it('finds every term, case and accents ignored, in code points of the original', () => {
    expect(highlightRanges('Été chaud, été', ['ete'])).toEqual([[0, 3], [11, 14]])
    expect(highlightRanges('cœur', ['oe'])).toEqual([[1, 2]])
    expect(highlightRanges('😀 Prépa', ['prepa'])).toEqual([[2, 7]])
  })

  it('merges overlapping matches and skips empty terms', () => {
    expect(highlightRanges('abcabc', ['abc', 'bca'])).toEqual([[0, 6]])
    expect(highlightRanges('abc', ['', ' '])).toEqual([])
    expect(highlightRanges('rien', ['x'])).toEqual([])
  })
})

describe('splitByRanges', () => {
  it('alternates plain and highlighted parts', () => {
    expect(splitByRanges('Préparer l’été', [[0, 3], [11, 14]])).toEqual([
      { text: 'Pré', match: true },
      { text: 'parer l’', match: false },
      { text: 'été', match: true },
    ])
  })

  it('clips ranges past the end and merges unsorted ones', () => {
    expect(splitByRanges('😀ab', [[2, 9], [0, 1]])).toEqual([
      { text: '😀', match: true },
      { text: 'a', match: false },
      { text: 'b', match: true },
    ])
    expect(splitByRanges('abc', [])).toEqual([{ text: 'abc', match: false }])
    expect(splitByRanges('', [[0, 2]])).toEqual([])
  })
})
