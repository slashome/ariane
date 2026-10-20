import { useQueryClient } from '@tanstack/react-query'
import { useEffect } from 'react'
import { subscribeToChanges } from '@/shared/api'

export function LiveUpdates() {
  const queryClient = useQueryClient()
  useEffect(() => subscribeToChanges(() => void queryClient.invalidateQueries()), [queryClient])
  return null
}
