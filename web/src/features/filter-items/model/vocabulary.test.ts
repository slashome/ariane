import { parseQuery } from './query'
import { checkQuery, rankOrders } from './vocabulary'
import { vocabulary } from './vocabulary.fixture'

describe('buildVocabulary', () => {
  it('gathers the statuses of every workflow once, the schema values in order and every handle', () => {
    expect(vocabulary.status.map((term) => term.value)).toEqual(['a-faire', 'fait', 'open', 'settled'])
    expect(vocabulary.people).toEqual([
      { value: 'andrea', label: 'Andrea Martel' },
      { value: 'keyvan', label: 'Keyvan Coen' },
    ])
    expect(rankOrders(vocabulary)).toEqual({ size: ['XS', 'S', 'M', 'L', 'XL'], impact: ['low', 'medium', 'high'], priority: ['low', 'medium', 'high'] })
  })
})

describe('checkQuery', () => {
  const messages = (text: string) => checkQuery(parseQuery(text), vocabulary).map((issue) => [issue.start, issue.severity, issue.message])

  it('accepts known keys and values, whatever their case and accents', () => {
    expect(messages('status:A-FAIRE category:todo is:Closed size:<=m @Keyvan free text')).toEqual([])
  })

  it('reports an unknown key, an unknown value, a missing value and a forbidden comparison', () => {
    const found = messages('foo:bar status:zzz,fait is: @bob category:>todo')
    expect(found.map(([start, severity]) => [start, severity])).toEqual([
      [0, 'warning'],
      [8, 'error'],
      [24, 'warning'],
      [28, 'error'],
      [33, 'error'],
    ])
    expect(found.map(([, , message]) => message)).toEqual([
      expect.stringContaining('searched as text'),
      expect.stringContaining('no status “zzz”'),
      expect.stringContaining('needs a value'),
      expect.stringContaining('@bob'),
      expect.stringContaining('cannot compare'),
    ])
  })
})
