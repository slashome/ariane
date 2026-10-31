import type { QueryClient } from '@tanstack/react-query'
import { queryKeys, syncMonday, type Item, type NodeSummary } from '@/shared/api'
import { toast } from '@/shared/ui'
import { syncMessage } from '../model/sync-message'

export function followMonday(queryClient: QueryClient, updated: Item) {
  const followed = updated.external.some((entry) => entry.startsWith('monday:'))
  const bound = queryClient
    .getQueryData<NodeSummary[]>(queryKeys.nodes)
    ?.some((node) => node.path === updated.node && node.tracker?.kind === 'monday')
  if (!followed && bound) {
    toast.info(`${updated.reference ?? updated.path}: no monday ticket linked, monday not updated`)
  }
  if (followed) {
    syncMonday(updated.path)
      .then((report) => {
        const message = syncMessage(report)
        if (message) {
          toast[message.tone](message.text)
        }
      })
      .catch((error: unknown) => toast.warning(`monday not updated: ${error instanceof Error ? error.message : String(error)}`))
  }
}
