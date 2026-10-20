import { useQuery } from '@tanstack/react-query'
import { fetchRepo, queryKeys } from '@/shared/api'

export function useRepo() {
  return useQuery({ queryKey: queryKeys.repo, queryFn: fetchRepo })
}
