export type DiffLineKind = 'added' | 'removed' | 'hunk' | 'meta' | 'context'

export interface DiffLine {
  kind: DiffLineKind
  text: string
}

// Headers (`diff`, `index`, `---`, `+++`…) precede the first `@@`; inside a hunk, `+` and `-` are content,
// even a removed line that itself starts with `--`.
export function diffLines(diff: string): DiffLine[] {
  if (diff === '') {
    return []
  }
  const texts = diff.endsWith('\n') ? diff.slice(0, -1).split('\n') : diff.split('\n')
  let inHunk = false
  return texts.map((text) => {
    if (text.startsWith('diff ')) {
      inHunk = false
    } else if (text.startsWith('@@')) {
      inHunk = true
      return { kind: 'hunk', text }
    }
    if (!inHunk || text.startsWith('\\')) {
      return { kind: 'meta', text }
    }
    if (text.startsWith('+')) {
      return { kind: 'added', text }
    }
    if (text.startsWith('-')) {
      return { kind: 'removed', text }
    }
    return { kind: 'context', text }
  })
}
