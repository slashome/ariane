import { useQuery } from '@tanstack/react-query'
import { fetchDiff, queryKeys } from '@/shared/api'

export function useDiff(path: string | null) {
  return useQuery({
    queryKey: queryKeys.diff(path ?? ''),
    queryFn: () => fetchDiff(path ?? ''),
    enabled: path !== null,
  })
}
