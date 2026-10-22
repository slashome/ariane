import { hostOf, linksIn, resolveLink } from './repo-path'

describe('resolveLink', () => {
  it('resolves a relative link from the file that holds it', () => {
    expect(resolveLink('tree/p/tasks/T1-a.md', '../stories/S1-s.md')).toBe('tree/p/stories/S1-s.md')
    expect(resolveLink('tree/p/INDEX.md', './tasks/T1-a.md#plan')).toBe('tree/p/tasks/T1-a.md')
    expect(resolveLink('tree/p/tasks/T1-a.md', 'T2%20b.md')).toBe('tree/p/tasks/T2 b.md')
  })

  it('leaves external, absolute and anchor links alone', () => {
    expect(resolveLink('tree/p/a.md', 'https://x.io/a.md')).toBeNull()
    expect(resolveLink('tree/p/a.md', 'mailto:me@x.io')).toBeNull()
    expect(resolveLink('tree/p/a.md', '#section')).toBeNull()
    expect(resolveLink('a.md', '../../b.md')).toBeNull()
  })
})

describe('links of a text', () => {
  it('are listed once, without trailing punctuation, with their host', () => {
    const text = 'See https://www.example.com/a. Then <https://younesaeo.com> and https://www.example.com/a again.'
    expect(linksIn(text)).toEqual(['https://www.example.com/a', 'https://younesaeo.com'])
    expect(hostOf('https://www.example.com/a')).toBe('example.com')
    expect(hostOf('nope')).toBeNull()
  })
})
