import { useMutation, useQueryClient } from '@tanstack/react-query'
import { ApiError, patchItem, queryKeys, type Item, type ItemChanges } from '@/shared/api'
import { toast } from '@/shared/ui'

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
    onSuccess: (updated) => {
      queryClient.setQueryData<Item[]>(queryKeys.items, (items) =>
        items?.map((entry) => (entry.path === updated.path ? updated : entry)),
      )
      void queryClient.invalidateQueries({ queryKey: queryKeys.repo })
    },
  })
}
