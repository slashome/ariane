import { hostOf, linksIn, highlightRanges } from '@/shared/lib'
import type { Item } from '@/shared/api'

export interface Link {
  url: string
  host: string
}

// The external links a ressource points to: those of its title, then of its body.
export function linksOf(item: Item): Link[] {
  const urls = linksIn(`${item.title ?? ''}\n${item.body ?? ''}`)
  return urls.flatMap((url) => {
    const host = hostOf(url)
    return host === null ? [] : [{ url, host }]
  })
}

// The text around the first match of the search words in the body, to show why a ressource matched;
// `null` without words or when they match elsewhere.
export function snippet(body: string | undefined, terms: readonly string[], radius = 70): string | null {
  if (!body) {
    return null
  }
  const flat = body.replace(/\s+/g, ' ')
  const [first] = highlightRanges(flat, terms)
  if (first === undefined) {
    return null
  }
  const characters = Array.from(flat)
  const start = Math.max(0, first[0] - radius)
  const end = Math.min(characters.length, first[1] + radius)
  return `${start > 0 ? '…' : ''}${characters.slice(start, end).join('').trim()}${end < characters.length ? '…' : ''}`
}
