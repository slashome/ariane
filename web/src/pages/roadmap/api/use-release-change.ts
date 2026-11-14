import { useMutation, useQueryClient } from '@tanstack/react-query'
import { addRelease, changeRelease, queryKeys, removeRelease, type ReleaseChange } from '@/shared/api'
import { toast } from '@/shared/ui'

export type ReleaseAction = 'add' | 'change' | 'remove'

export interface ReleaseRequest {
  action: ReleaseAction
  change: ReleaseChange
}

function send({ action, change }: ReleaseRequest) {
  switch (action) {
    case 'add':
      return addRelease(change)
    case 'change':
      return changeRelease(change)
    case 'remove':
      return removeRelease({ node: change.node, name: change.name })
  }
}

export function useReleaseChange() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: send,
    onSuccess: () => {
      for (const queryKey of [queryKeys.nodes, ['roadmap'], queryKeys.items, queryKeys.repo]) {
        void queryClient.invalidateQueries({ queryKey })
      }
    },
    onError: (error) => {
      toast.error(`Could not change the releases: ${error.message}`)
    },
  })
}
