import { fold, foldText } from './fold'

// A range of a text to highlight, `[start, end)` in code points, as `ariane search` gives them.
export type Range = readonly [number, number]

export interface Segment {
  text: string
  match: boolean
}

// Where the terms occur in a text, case and accents ignored: sorted ranges, overlapping ones merged.
export function highlightRanges(text: string, terms: readonly string[]): Range[] {
  const { text: folded, origin } = fold(text)
  const found: [number, number][] = []
  for (const term of terms.map(foldText).filter((term) => term.trim() !== '')) {
    for (let index = folded.indexOf(term); index !== -1; index = folded.indexOf(term, index + 1)) {
      found.push([origin[index] ?? 0, (origin[index + term.length - 1] ?? 0) + 1])
    }
  }
  return merge(found)
}

function merge(ranges: [number, number][]): Range[] {
  const sorted = [...ranges].sort((left, right) => left[0] - right[0] || left[1] - right[1])
  const merged: [number, number][] = []
  for (const [start, end] of sorted) {
    const last = merged[merged.length - 1]
    if (last !== undefined && start <= last[1]) {
      last[1] = Math.max(last[1], end)
    } else {
      merged.push([start, end])
    }
  }
  return merged
}

// Cuts a text into the parts to highlight and the parts between them; ranges past the end are clipped.
export function splitByRanges(text: string, ranges: readonly Range[]): Segment[] {
  const characters = Array.from(text)
  const segments: Segment[] = []
  let cursor = 0
  for (const [start, end] of merge(ranges.map(([start, end]) => [start, end]))) {
    const from = Math.max(start, cursor)
    const to = Math.min(end, characters.length)
    if (from >= to) {
      continue
    }
    if (from > cursor) {
      segments.push({ text: characters.slice(cursor, from).join(''), match: false })
    }
    segments.push({ text: characters.slice(from, to).join(''), match: true })
    cursor = to
  }
  if (cursor < characters.length) {
    segments.push({ text: characters.slice(cursor).join(''), match: false })
  }
  return segments
}
