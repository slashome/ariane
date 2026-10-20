import { useQuery } from '@tanstack/react-query'
import { fetchItems, queryKeys } from '@/shared/api'

export function useItems() {
  return useQuery({ queryKey: queryKeys.items, queryFn: fetchItems })
}
