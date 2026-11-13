import { useQuery } from '@tanstack/react-query'
import { fetchRoadmap, queryKeys } from '@/shared/api'

export function useRoadmap(node: string | null) {
  return useQuery({
    queryKey: queryKeys.roadmap(node ?? ''),
    queryFn: () => fetchRoadmap(node ?? ''),
    enabled: node !== null && node !== '',
  })
}
