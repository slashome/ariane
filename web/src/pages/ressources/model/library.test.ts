import { makeItem } from '@/shared/api'
import { linksOf, snippet } from './library'

describe('ressource library', () => {
  it('finds the external links of the title and the body', () => {
    const item = makeItem({ kind: 'ressource', title: 'Site — <https://younesaeo.com>', body: 'See https://github.com/a/b and https://younesaeo.com.' })
    expect(linksOf(item)).toEqual([
      { url: 'https://younesaeo.com', host: 'younesaeo.com' },
      { url: 'https://github.com/a/b', host: 'github.com' },
    ])
  })

  it('shows the text around a body match', () => {
    const body = `${'a '.repeat(60)}the needle is here ${'b '.repeat(60)}`
    expect(snippet(body, ['absent', 'NÉEDLE'], 10)).toBe('…a a a the needle is here b…')
    expect(snippet(body, ['absent'])).toBeNull()
    expect(snippet(body, [' '])).toBeNull()
  })
})
