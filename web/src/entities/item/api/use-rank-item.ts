import { useMutation, useQueryClient } from '@tanstack/react-query'
import { ApiError, queryKeys, rankItem, type Category, type Item, type Place } from '@/shared/api'
import { toast } from '@/shared/ui'
import { placeInOrder } from '../model/manual-order'
import { followMonday } from './follow-monday'

export interface ItemMove {
  item: Item
  place: Place
  other?: Item
  status?: { to: string; category: Category | null }
}

function seenOf({ item, other }: ItemMove): Record<string, number | null> {
  return other === undefined ? { [item.path]: item.rank } : { [item.path]: item.rank, [other.path]: other.rank }
}

function changedBy({ item, status }: ItemMove): Partial<Item> {
  if (status === undefined) {
    return {}
  }
  const leavesWaiting = item.category === 'waiting' && status.category !== 'waiting'
  return {
    status: status.to,
    category: status.category,
    ...(leavesWaiting ? { waiting_for: [], waiting_on: null, waiting_since: null } : {}),
  }
}

export function useRankItem() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (move: ItemMove) =>
      rankItem({
        path: move.item.path,
        place: move.place,
        other: move.other?.path,
        seen: seenOf(move),
        status: move.status === undefined ? undefined : { from: move.item.status, to: move.status.to },
      }),
    onMutate: async (move) => {
      await queryClient.cancelQueries({ queryKey: queryKeys.items })
      const previous = queryClient.getQueryData<Item[]>(queryKeys.items)
      queryClient.setQueryData<Item[]>(queryKeys.items, (items) =>
        items === undefined ? items : placeInOrder(items, move.item, move.place, move.other ?? null, changedBy(move)),
      )
      return { previous }
    },
    onError: (error, _move, context) => {
      if (context?.previous) {
        queryClient.setQueryData(queryKeys.items, context.previous)
      }
      if (error instanceof ApiError && error.status === 409) {
        toast.warning('Changed on disk, reloaded')
        void queryClient.invalidateQueries({ queryKey: queryKeys.items })
        return
      }
      toast.error(`Could not move: ${error.message}`)
    },
    onSuccess: ({ items: updated }, move) => {
      const byPath = new Map(updated.map((item) => [item.path, item]))
      queryClient.setQueryData<Item[]>(queryKeys.items, (items) =>
        items?.map((entry) => {
          const fresh = byPath.get(entry.path)
          return fresh === undefined ? entry : { ...entry, ...fresh }
        }),
      )
      void queryClient.invalidateQueries({ queryKey: queryKeys.items })
      void queryClient.invalidateQueries({ queryKey: queryKeys.repo })
      void queryClient.invalidateQueries({ queryKey: queryKeys.itemDetails })
      const moved = byPath.get(move.item.path)
      if (move.status !== undefined && moved !== undefined) {
        followMonday(queryClient, moved)
      }
    },
  })
}

export function useMoveItem() {
  const rank = useRankItem()
  return (item: Item, over: Item, place: 'before' | 'after') => {
    if (item.node !== over.node || item.kind !== over.kind) {
      toast.info('The order is per node and kind: drop it next to an item of its own node and kind')
      return
    }
    rank.mutate({ item, place, other: over })
  }
}
