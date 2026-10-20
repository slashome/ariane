import { useQuery } from '@tanstack/react-query'
import { fetchSchema, queryKeys } from '@/shared/api'

export function useSchema() {
  return useQuery({ queryKey: queryKeys.schema, queryFn: fetchSchema, staleTime: Infinity })
}
