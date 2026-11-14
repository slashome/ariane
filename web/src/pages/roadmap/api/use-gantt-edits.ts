import { useQueryClient } from '@tanstack/react-query'
import { changesFor, useRankItem, useUpdateItem } from '@/entities/item'
import type { Item } from '@/shared/api'
import type { Place, Size } from '../model/edits'

export interface GanttEdits {
  move: (item: Item, other: Item, place: Place) => void
  resize: (item: Item, size: Size) => void
}

export function useGanttEdits(): GanttEdits {
  const queryClient = useQueryClient()
  const rank = useRankItem()
  const update = useUpdateItem()
  const refresh = () => void queryClient.invalidateQueries({ queryKey: ['roadmap'] })
  return {
    move: (item, other, place) => rank.mutate({ item, place, other }, { onSuccess: refresh }),
    resize: (item, size) => {
      const changes = changesFor(item, { size })
      if (changes.size === undefined) {
        return
      }
      update.mutate({ item, changes, optimistic: { size } }, { onSuccess: refresh })
    },
  }
}
