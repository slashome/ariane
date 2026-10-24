import { useQuery } from '@tanstack/react-query'
import { fetchProxy, queryKeys } from '@/shared/api'

export function useProxy({ refetchInterval }: { refetchInterval?: number } = {}) {
  return useQuery({ queryKey: queryKeys.proxy, queryFn: fetchProxy, refetchInterval })
}
