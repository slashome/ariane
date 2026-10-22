import { excerpt } from './excerpt'

describe('excerpt', () => {
  it('takes the first paragraph of prose, links and emphasis flattened', () => {
    const body = '# Title\n\n> Opened on 2026-01-02\n\n- a list\n\nWe keep **one** [store](x.md) per entity.\n\nMore.'
    expect(excerpt(body)).toBe('We keep one store per entity.')
    expect(excerpt('# Only a title')).toBe('')
    expect(excerpt('word '.repeat(100), 20)).toHaveLength(20)
  })
})
