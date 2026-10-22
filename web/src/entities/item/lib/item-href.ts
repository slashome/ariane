import type { Item } from '@/shared/api'
import { itemPath } from '@/shared/config'

export function itemHref(item: Pick<Item, 'kind' | 'node' | 'id'>): string | null {
  return item.id === null ? null : itemPath(item.kind, item.node, item.id)
}
