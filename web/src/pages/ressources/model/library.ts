import { hostOf, linksIn } from '@/shared/lib'
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

// The text around the first match of `query` in the body, to show why a ressource matched; `null` when
// the query is empty or matches elsewhere.
export function snippet(body: string | undefined, query: string, radius = 70): string | null {
  const needle = query.trim().toLowerCase()
  if (!body || needle === '') {
    return null
  }
  const flat = body.replace(/\s+/g, ' ')
  const index = flat.toLowerCase().indexOf(needle)
  if (index < 0) {
    return null
  }
  const start = Math.max(0, index - radius)
  const end = Math.min(flat.length, index + needle.length + radius)
  return `${start > 0 ? '…' : ''}${flat.slice(start, end).trim()}${end < flat.length ? '…' : ''}`
}
