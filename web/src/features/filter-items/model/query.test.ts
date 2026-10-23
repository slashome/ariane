import { formatQuery, highlightTerms, parseQuery, tokenize } from './query'

describe('parseQuery', () => {
  it('reads filters, comparisons, people, negations, phrases and free words with their spans', () => {
    const { clauses } = parseQuery('status:todo,doing size:<=M -@keyvan "two words" lang')
    expect(clauses).toEqual([
      { start: 0, end: 17, negated: false, type: 'filter', key: 'status', comparator: '=', values: ['todo', 'doing'] },
      { start: 18, end: 26, negated: false, type: 'filter', key: 'size', comparator: '<=', values: ['M'] },
      { start: 27, end: 35, negated: true, type: 'person', handle: 'keyvan' },
      { start: 36, end: 47, negated: false, type: 'text', text: 'two words' },
      { start: 48, end: 52, negated: false, type: 'text', text: 'lang' },
    ])
  })

  it('keeps a key it does not know as text to report, and a key being typed without values', () => {
    expect(parseQuery('Foo:bar').clauses[0]).toMatchObject({ type: 'unknown', key: 'foo', text: 'Foo:bar' })
    expect(parseQuery('IS:').clauses[0]).toMatchObject({ type: 'filter', key: 'is', values: [] })
    expect(parseQuery('https://x.org').clauses[0]).toMatchObject({ type: 'unknown', key: 'https' })
    expect(parseQuery('- -"a b').clauses).toMatchObject([
      { type: 'text', text: '-', negated: false },
      { type: 'text', text: 'a b', negated: true },
    ])
  })

  it('writes a query back as the field reads it', () => {
    for (const text of ['status:todo,doing size:<=M -@keyvan "two words" lang', 'is:open -category:done x:y', '']) {
      expect(formatQuery(parseQuery(text))).toBe(text)
    }
    expect(formatQuery(parseQuery('  a    size:=S '))).toBe('a size:S')
  })

  it('tokenizes a quoted phrase whole, even unclosed', () => {
    expect(tokenize('a "b c" "d e')).toEqual([
      { start: 0, end: 1 },
      { start: 2, end: 7 },
      { start: 8, end: 12 },
    ])
  })

  it('highlights the free words, not the filters nor what is negated', () => {
    expect(highlightTerms(parseQuery('status:todo été -draft "deux mots" x:y @me'))).toEqual(['été', 'deux mots', 'x:y'])
  })
})
