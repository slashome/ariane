import { diffLines } from './diff-lines'

describe('diffLines', () => {
  it('tells added, removed, hunk, header and context lines apart', () => {
    const diff = [
      'diff --git a/tree/a.md b/tree/a.md',
      'index 1..2 100644',
      '--- a/tree/a.md',
      '+++ b/tree/a.md',
      '@@ -1,2 +1,2 @@',
      ' kept',
      '-title: A',
      '+title: B',
      '--- removed rule',
      '\\ No newline at end of file',
      '',
    ].join('\n')
    expect(diffLines(diff).map((line) => line.kind)).toEqual([
      'meta',
      'meta',
      'meta',
      'meta',
      'hunk',
      'context',
      'removed',
      'added',
      'removed',
      'meta',
    ])
  })

  it('gives no line for an empty diff', () => {
    expect(diffLines('')).toEqual([])
  })
})
