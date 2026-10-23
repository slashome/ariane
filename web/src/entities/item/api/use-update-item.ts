import { useMutation, useQueryClient } from '@tanstack/react-query'
import { ApiError, patchItem, queryKeys, syncMonday, type Item, type ItemChanges } from '@/shared/api'
import { toast } from '@/shared/ui'
import { syncMessage } from '../model/sync-message'

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
      const previous = queryClient.getQueryData<Item[]>(queryKeys.items)
      queryClient.setQueryData<Item[]>(queryKeys.items, (items) =>
        items?.map((entry) => (entry.path === item.path ? { ...entry, ...optimistic } : entry)),
      )
      return { previous }
    },
    onError: (error, _update, context) => {
      if (context?.previous) {
        queryClient.setQueryData(queryKeys.items, context.previous)
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
      void queryClient.invalidateQueries({ queryKey: queryKeys.repo })
      void queryClient.invalidateQueries({ queryKey: queryKeys.itemDetails })
      const followed = updated.external.some((entry) => entry.startsWith('monday:'))
      if (followed && Object.keys(changes).some((key) => synced.includes(key))) {
        syncMonday(updated.path)
          .then((report) => {
            const message = syncMessage(report)
            if (message) {
              toast[message.tone](message.text)
            }
          })
          .catch((error: unknown) => toast.warning(`monday not updated: ${error instanceof Error ? error.message : String(error)}`))
      }
    },
  })
}
