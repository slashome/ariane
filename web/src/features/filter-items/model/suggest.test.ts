import { applySuggestion, suggest, type Suggestions } from './suggest'
import { vocabulary } from './vocabulary.fixture'

function option(found: Suggestions, index: number) {
  const chosen = found.options[index]
  if (chosen === undefined) {
    throw new Error(`no suggestion ${index}`)
  }
  return chosen
}

const labels = (text: string, cursor = text.length) => suggest(text, cursor, vocabulary).options.map((option) => option.label)

describe('suggest', () => {
  it('proposes the keys first, and nothing for free words', () => {
    expect(labels('')).toEqual(['status:', 'category:', 'is:', 'size:', 'impact:', 'priority:', '@person'])
    expect(labels('fix s')).toEqual(['status:', 'size:'])
    expect(labels('lang')).toEqual([])
    expect(labels('"s')).toEqual([])
  })

  it('then the known values of the key, after a comma or a comparison too', () => {
    expect(labels('status:')).toEqual(['a-faire', 'fait', 'open', 'settled'])
    expect(labels('status:a-faire,f')).toEqual(['fait'])
    expect(labels('is:')).toEqual(['open', 'closed'])
    expect(labels('size:')).toEqual(['XS', 'S', 'M', 'L', 'XL', 'size:<=', 'size:>='])
    expect(labels('-size:<=x')).toEqual(['XS', 'XL'])
    expect(labels('category:fai')).toEqual([])
    expect(labels('nope:')).toEqual([])
  })

  it('proposes the people after @, by handle or name', () => {
    expect(labels('@')).toEqual(['@andrea', '@keyvan'])
    expect(labels('-@coen')).toEqual(['@keyvan'])
  })

  it('replaces the token under the cursor and moves past it', () => {
    const text = 'stat fix'
    const found = suggest(text, 2, vocabulary)
    const key = applySuggestion(text, found, option(found, 0))
    expect(key).toEqual({ text: 'status: fix', cursor: 7 })
    const values = suggest('status:a fix', 8, vocabulary)
    expect(applySuggestion('status:a fix', values, option(values, 0))).toEqual({ text: 'status:a-faire fix', cursor: 15 })
    const last = suggest('size:<=x', 8, vocabulary)
    expect(applySuggestion('size:<=x', last, option(last, 1))).toEqual({ text: 'size:<=XL ', cursor: 10 })
  })
})
