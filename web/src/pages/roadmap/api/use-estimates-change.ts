import { useMutation, useQueryClient } from '@tanstack/react-query'
import { changeEstimates, queryKeys } from '@/shared/api'
import { toast } from '@/shared/ui'

export function useEstimatesChange() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: changeEstimates,
    onSuccess: () => {
      for (const queryKey of [queryKeys.nodes, ['roadmap'], queryKeys.items, queryKeys.itemDetails, queryKeys.repo]) {
        void queryClient.invalidateQueries({ queryKey })
      }
    },
    onError: (error) => {
      toast.error(`Could not change the estimates: ${error.message}`)
    },
  })
}
