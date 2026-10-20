import { useQuery } from '@tanstack/react-query'
import { fetchNodes, queryKeys } from '@/shared/api'

export function useNodes() {
  return useQuery({ queryKey: queryKeys.nodes, queryFn: fetchNodes })
}
