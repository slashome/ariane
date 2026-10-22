import { useQuery } from '@tanstack/react-query'
import { fetchItem, fetchItemsWithBody, queryKeys, type Kind } from '@/shared/api'

export function useItem(reference: string | null) {
  return useQuery({
    queryKey: queryKeys.item(reference ?? ''),
    queryFn: () => fetchItem(reference ?? ''),
    enabled: reference !== null,
  })
}

// The items of one kind with their Markdown body, for the views that read or search it.
export function useItemsWithBody(kind: Kind) {
  return useQuery({ queryKey: queryKeys.itemsWithBody(kind), queryFn: () => fetchItemsWithBody(kind) })
}
