import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { fetchSearch, queryKeys } from '@/shared/api'

// The items `ariane search` finds for a text, best first; nothing is asked below two characters.
export function useSearch(text: string, archived: boolean) {
  const q = text.trim()
  return useQuery({
    queryKey: queryKeys.search(q, archived),
    queryFn: () => fetchSearch(q, archived),
    enabled: q.length >= 2,
    placeholderData: keepPreviousData,
  })
}
