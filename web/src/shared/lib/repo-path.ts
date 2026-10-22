// Resolves a link written in the Markdown file `from` (a path relative to the repository root, such as
// `tree/p/tasks/T1-a.md`) into a path relative to the root; `null` for an external or anchor-only link,
// or one that climbs above the root.
export function resolveLink(from: string, href: string): string | null {
  if (href === '' || href.startsWith('#') || /^[a-z][a-z0-9+.-]*:/i.test(href) || href.startsWith('/')) {
    return null
  }
  const target = decodeURIComponent(href.split('#')[0] ?? '')
  const parts = from.split('/').slice(0, -1)
  for (const segment of target.split('/')) {
    if (segment === '' || segment === '.') {
      continue
    }
    if (segment === '..') {
      if (parts.length === 0) {
        return null
      }
      parts.pop()
    } else {
      parts.push(segment)
    }
  }
  return parts.join('/')
}

export function isExternalLink(href: string): boolean {
  return /^https?:\/\//i.test(href)
}

export function hostOf(url: string): string | null {
  try {
    return new URL(url).hostname.replace(/^www\./, '')
  } catch {
    return null
  }
}

// Every http(s) URL written in a Markdown text, in order of appearance, without duplicates.
export function linksIn(text: string): string[] {
  const found = text.match(/https?:\/\/[^\s)<>\]"'`]+/g) ?? []
  return [...new Set(found.map((url) => url.replace(/[.,;:!?]+$/, '')))]
}
