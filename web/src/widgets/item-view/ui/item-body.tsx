import { ExternalLink } from 'lucide-react'
import type { ReactNode } from 'react'
import { ItemLink } from '@/entities/item'
import type { Item } from '@/shared/api'
import { isExternalLink, resolveLink } from '@/shared/lib'
import { Markdown } from '@/shared/ui'

// The Markdown body of an item, its relative links to other items turned into routes of the interface.
export function ItemBody({ item, body, items }: { item: Item; body: string; items: Item[] }) {
  const renderLink = (href: string, children: ReactNode) => {
    if (isExternalLink(href)) {
      return (
        <a href={href} target="_blank" rel="noreferrer">
          {children}
          <ExternalLink className="ml-0.5 inline size-3 align-baseline" aria-hidden />
        </a>
      )
    }
    const path = resolveLink(item.path, href)
    const target = path === null ? undefined : items.find((entry) => entry.path === path)
    if (target) {
      return (
        <ItemLink item={target} className="text-primary">
          {children}
        </ItemLink>
      )
    }
    return (
      <span className="text-muted-foreground underline decoration-dotted" title={path ?? href}>
        {children}
      </span>
    )
  }
  return body.trim() === '' ? (
    <p className="text-sm text-muted-foreground">This item has no body.</p>
  ) : (
    <Markdown source={body} renderLink={renderLink} />
  )
}
