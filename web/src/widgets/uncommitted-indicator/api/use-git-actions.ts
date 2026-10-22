import { useMutation, useQueryClient } from '@tanstack/react-query'
import { ApiError, commitChanges, discardChanges, pushChanges, queryKeys } from '@/shared/api'
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

export function useDiscard() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (paths?: string[]) => discardChanges(paths),
    onSuccess: ({ discarded }) => {
      toast.success(`Discarded ${discarded.length} ${discarded.length === 1 ? 'change' : 'changes'}`)
    },
    onError: (error) => {
      if (error instanceof ApiError && error.status === 409) {
        toast.info('Nothing to discard')
        return
      }
      toast.error(`Discard failed: ${error.message}`)
    },
    onSettled: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.repo })
      void queryClient.invalidateQueries({ queryKey: queryKeys.items })
      void queryClient.invalidateQueries({ queryKey: queryKeys.diffs })
    },
  })
}
