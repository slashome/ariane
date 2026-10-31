import type { ReactNode } from 'react'
import { Link, useLocation } from 'react-router'
import type { Item } from '@/shared/api'
import { scopeSearch } from '@/shared/config'
import { cn } from '@/shared/lib'
import { itemHref } from '../lib/item-href'
import './item.css'

// A link to an item's page, keeping the node scope and the archive toggle of the URL.
export function ItemLink({
  item,
  children,
  className,
}: {
  item: Pick<Item, 'kind' | 'node' | 'id'>
  children?: ReactNode
  className?: string
}) {
  const { search } = useLocation()
  const href = itemHref(item)
  const content = children ?? item.id
  if (href === null) {
    return <span className={className}>{content}</span>
  }
  return (
    <Link to={{ pathname: href, search: scopeSearch(search) }} className={cn('item-link', className)}>
      {content}
    </Link>
  )
}
