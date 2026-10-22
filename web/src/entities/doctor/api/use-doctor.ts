import { useQuery } from '@tanstack/react-query'
import { fetchDoctor, queryKeys } from '@/shared/api'

export function useDoctor() {
  return useQuery({ queryKey: queryKeys.doctor, queryFn: fetchDoctor })
}
