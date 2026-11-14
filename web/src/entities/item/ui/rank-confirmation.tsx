import { useState, type ReactNode } from 'react'
import type { Item, Place } from '@/shared/api'
import { ConfirmDialog } from '@/shared/ui'
import { itemsRankedByMove } from '../model/manual-order'

interface Held {
  count: number
  send: () => void
}

export interface RankConfirmation {
  confirm: (moved: Item, place: Place, other: Item | null, send: () => void) => void
  dialog: ReactNode
}

export function useRankConfirmation(items: Item[]): RankConfirmation {
  const [held, setHeld] = useState<Held | null>(null)
  const confirm = (moved: Item, place: Place, other: Item | null, send: () => void) => {
    const count = itemsRankedByMove(items, moved, place, other)
    if (count === 0) {
      send()
      return
    }
    setHeld({ count, send })
  }
  const dialog = (
    <ConfirmDialog
      open={held !== null}
      onOpenChange={(open) => {
        if (!open) {
          setHeld(null)
        }
      }}
      title="Rank the whole node?"
      description={`This move ranks the ${held?.count ?? 0} items of this node: ${held?.count ?? 0} files modified, not committed. Continue?`}
      confirmLabel="Continue"
      tone="default"
      onConfirm={() => {
        held?.send()
        setHeld(null)
      }}
    />
  )
  return { confirm, dialog }
}
