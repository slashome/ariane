import { Send } from 'lucide-react'
import type { Item } from '@/shared/api'
import { Button } from '@/shared/ui'
import { useSyncItem } from '../api/use-sync-item'
import { mondayLinks } from '../model/bound'

// Sends the item's status to its monday ticket now; nothing for an item bound to none.
export function SyncItemButton({ item }: { item: Item }) {
  const sync = useSyncItem()
  if (mondayLinks(item).length === 0) {
    return null
  }
  return (
    <Button size="sm" variant="outline" disabled={sync.isPending} onClick={() => sync.mutate(item)} title="Send the status to the monday ticket, refusing on a conflict">
      <Send className="size-3.5" aria-hidden />
      {sync.isPending ? 'Sending…' : 'Send to monday'}
    </Button>
  )
}
