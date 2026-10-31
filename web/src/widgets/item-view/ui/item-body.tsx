import { useLocation, useNavigate } from 'react-router'
import { itemHref } from '@/entities/item'
import type { Item } from '@/shared/api'
import { scopeSearch } from '@/shared/config'
import { isExternalLink, resolveLink } from '@/shared/lib'
import { Markdown, type LinkTarget } from '@/shared/ui'

export function ItemBody({ item, body, items }: { item: Item; body: string; items: Item[] }) {
  const { search } = useLocation()
  const navigate = useNavigate()
  const linkTarget = (href: string): LinkTarget => {
    if (isExternalLink(href)) {
      return { kind: 'external', href }
    }
    const path = resolveLink(item.path, href)
    const target = path === null ? undefined : items.find((entry) => entry.path === path)
    const route = target ? itemHref(target) : null
    return route === null ? { kind: 'unresolved', title: path ?? href } : { kind: 'internal', href: `${route}${scopeSearch(search)}` }
  }
  return body.trim() === '' ? (
    <p className="text-sm text-muted-foreground">This item has no body.</p>
  ) : (
    <Markdown source={body} resolveLink={linkTarget} onNavigate={(href) => void navigate(href)} />
  )
}
