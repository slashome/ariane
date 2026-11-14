import { useMutation, useQueryClient } from '@tanstack/react-query'
import { ApiError, patchItem, queryKeys, type Item, type ItemChanges, type ItemDetail } from '@/shared/api'
import { toast } from '@/shared/ui'
import { followMonday } from './follow-monday'

// The changes a monday ticket follows: the status, and whom it waits for.
const synced = ['status', 'waiting_for', 'waiting_on']

export interface ItemUpdate {
  item: Item
  changes: ItemChanges
  optimistic: Partial<Item>
}

export function useUpdateItem() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ item, changes }: ItemUpdate) => patchItem({ path: item.path, changes }),
    onMutate: async ({ item, optimistic }) => {
      await queryClient.cancelQueries({ queryKey: queryKeys.items })
      await queryClient.cancelQueries({ queryKey: queryKeys.itemDetails })
      const previous = queryClient.getQueryData<Item[]>(queryKeys.items)
      const previousDetails = queryClient.getQueriesData<ItemDetail>({ queryKey: queryKeys.itemDetails })
      queryClient.setQueryData<Item[]>(queryKeys.items, (items) =>
        items?.map((entry) => (entry.path === item.path ? { ...entry, ...optimistic } : entry)),
      )
      queryClient.setQueriesData<ItemDetail>({ queryKey: queryKeys.itemDetails }, (detail) =>
        detail?.path === item.path ? { ...detail, ...optimistic } : detail,
      )
      return { previous, previousDetails }
    },
    onError: (error, _update, context) => {
      if (context?.previous) {
        queryClient.setQueryData(queryKeys.items, context.previous)
      }
      for (const [key, detail] of context?.previousDetails ?? []) {
        queryClient.setQueryData(key, detail)
      }
      if (error instanceof ApiError && error.status === 409) {
        toast.warning('Changed on disk, reloaded')
        void queryClient.invalidateQueries({ queryKey: queryKeys.items })
        return
      }
      toast.error(`Could not save: ${error.message}`)
    },
    onSuccess: (updated, { changes }) => {
      queryClient.setQueryData<Item[]>(queryKeys.items, (items) =>
        items?.map((entry) => (entry.path === updated.path ? { ...entry, ...updated } : entry)),
      )
      queryClient.setQueriesData<ItemDetail>({ queryKey: queryKeys.itemDetails }, (detail) =>
        detail?.path === updated.path ? { ...detail, ...updated } : detail,
      )
      void queryClient.invalidateQueries({ queryKey: queryKeys.repo })
      void queryClient.invalidateQueries({ queryKey: queryKeys.itemDetails })
      if (!Object.keys(changes).some((key) => synced.includes(key))) {
        return
      }
      followMonday(queryClient, updated)
    },
  })
}
