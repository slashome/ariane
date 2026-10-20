import { useMutation, useQueryClient } from '@tanstack/react-query'
import { ApiError, commitChanges, pushChanges, queryKeys } from '@/shared/api'
import { toast } from '@/shared/ui'

export function useCommit() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (message: string) => commitChanges(message),
    onSuccess: ({ commit }) => {
      toast.success(`Committed ${commit.slice(0, 7)}`)
      void queryClient.invalidateQueries({ queryKey: queryKeys.repo })
    },
    onError: (error) => {
      if (error instanceof ApiError && error.status === 409) {
        toast.info('Nothing to commit')
        void queryClient.invalidateQueries({ queryKey: queryKeys.repo })
        return
      }
      toast.error(`Commit failed: ${error.message}`)
    },
  })
}

export function usePush() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: pushChanges,
    onSuccess: () => {
      toast.success('Pushed')
      void queryClient.invalidateQueries({ queryKey: queryKeys.repo })
    },
    onError: (error) => toast.error(`Push failed: ${error.message}`),
  })
}
