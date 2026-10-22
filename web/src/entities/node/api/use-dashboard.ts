import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { fetchDashboard, queryKeys, saveDashboard, type Dashboard } from '@/shared/api'
import { toast } from '@/shared/ui'

export function useDashboard() {
  return useQuery({ queryKey: queryKeys.dashboard, queryFn: fetchDashboard })
}

export function useSaveDashboard() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (dashboard: Dashboard) => saveDashboard(dashboard),
    onMutate: async (dashboard) => {
      await queryClient.cancelQueries({ queryKey: queryKeys.dashboard })
      const previous = queryClient.getQueryData<Dashboard>(queryKeys.dashboard)
      queryClient.setQueryData(queryKeys.dashboard, dashboard)
      return { previous }
    },
    onError: (error, _dashboard, context) => {
      if (context?.previous) {
        queryClient.setQueryData(queryKeys.dashboard, context.previous)
      }
      toast.error(`Could not save the dashboard: ${error.message}`)
    },
    onSuccess: () => {
      toast.success('Dashboard saved')
      void queryClient.invalidateQueries({ queryKey: queryKeys.repo })
    },
  })
}
