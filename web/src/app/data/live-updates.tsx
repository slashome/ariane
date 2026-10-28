import { useQueryClient } from '@tanstack/react-query'
import { useEffect } from 'react'
import { queryKeys, subscribeToChanges } from '@/shared/api'

// The schema is compiled into the server, and the proxy report refetches on its
// own interval: a change of the content touches neither.
const unaffected: readonly unknown[] = [queryKeys.schema[0], queryKeys.proxy[0]]

export function LiveUpdates() {
  const queryClient = useQueryClient()
  useEffect(
    () =>
      subscribeToChanges(
        () => void queryClient.invalidateQueries({ predicate: (query) => !unaffected.includes(query.queryKey[0]) }),
      ),
    [queryClient],
  )
  return null
}
