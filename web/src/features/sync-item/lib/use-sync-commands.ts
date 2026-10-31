import { useItems } from '@/entities/item'
import type { Command } from '@/shared/lib'
import { useSyncItem } from '../api/use-sync-item'
import { mondayLinks } from '../model/bound'

// “Send an item's status to monday…”, which lists the live items bound to a monday ticket.
export function useSyncCommands(): Command[] {
  const { data: items = [] } = useItems()
  const sync = useSyncItem()
  const bound = items.filter((item) => !item.archived && mondayLinks(item).length > 0)
  return [
    {
      id: 'sync:monday',
      group: 'Actions',
      title: 'Send an item’s status to monday…',
      icon: 'send',
      keywords: ['monday', 'sync', 'tracker'],
      hint: `${bound.length} items`,
      children: bound.map((item) => ({
        id: `sync:monday:${item.path}`,
        group: 'Items bound to monday',
        title: item.title ?? item.path,
        hint: mondayLinks(item).join(' '),
        keywords: [item.id, item.reference, ...item.external].filter((word): word is string => word !== null),
        icon: 'refresh',
        run: () => sync.mutate(item),
      })),
    },
  ]
}
