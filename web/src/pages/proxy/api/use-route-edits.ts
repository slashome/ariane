import { useMutation, useQueryClient } from '@tanstack/react-query'
import { queryKeys, removeRoute, setRoute, type RouteChange, type RouteTable } from '@/shared/api'
import { toast } from '@/shared/ui'

function useRouteEdit<T>(send: (input: T) => Promise<RouteTable>, done: string) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: send,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: queryKeys.proxy })
      void queryClient.invalidateQueries({ queryKey: queryKeys.nodes })
      void queryClient.invalidateQueries({ queryKey: queryKeys.repo })
      toast.success(done)
    },
    onError: (error) => toast.error(error.message),
  })
}

export const useSetRoute = () => useRouteEdit((route: RouteChange) => setRoute(route), 'Route saved in node.toml')
export const useRemoveRoute = () => useRouteEdit((node: string) => removeRoute(node), 'Route removed from node.toml')
