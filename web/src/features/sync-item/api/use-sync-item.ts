import { useMutation } from '@tanstack/react-query'
import { syncMessage } from '@/entities/item'
import { syncMonday, type Item } from '@/shared/api'
import { toast } from '@/shared/ui'

// Sends one item's status to its monday ticket and tells what happened: moved, already in step, or why not.
export function useSyncItem() {
  return useMutation({
    mutationFn: (item: Item) => syncMonday(item.path),
    onSuccess: (report, item) => {
      const message = syncMessage(report)
      if (message === null) {
        toast.success(`${item.reference ?? item.path}: monday already in step`)
      } else {
        toast[message.tone](message.text)
      }
    },
    onError: (error, item) => toast.error(`${item.reference ?? item.path}: monday not updated — ${error.message}`),
  })
}
