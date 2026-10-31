import type { Item } from '@/shared/api'
import { Button, Icon } from '@/shared/ui'
import { useSyncItem } from '../api/use-sync-item'
import { mondayLinks } from '../model/bound'
import './sync-item.css'

// Sends the item's status to its monday ticket now; nothing for an item bound to none.
export function SyncItemButton({ item }: { item: Item }) {
  const sync = useSyncItem()
  if (mondayLinks(item).length === 0) {
    return null
  }
  return (
    <Button size="sm" variant="outline" disabled={sync.isPending} onClick={() => sync.mutate(item)} title="Send the status to the monday ticket, refusing on a conflict">
      <Icon name="send" className="sync-item-button__icon" />
      {sync.isPending ? 'Sending…' : 'Send to monday'}
    </Button>
  )
}
