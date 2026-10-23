import type { Item } from '@/shared/api'

// The monday tickets an item is bound to, `monday:<id>` in its `external`.
export function mondayLinks(item: Item): string[] {
  return item.external.filter((link) => link.startsWith('monday:'))
}
