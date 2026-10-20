import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useState, type ReactNode } from 'react'
import { LiveUpdates } from './live-updates'

export function AppProviders({ children }: { children: ReactNode }) {
  const [queryClient] = useState(
    () => new QueryClient({ defaultOptions: { queries: { staleTime: 30_000, refetchOnWindowFocus: false } } }),
  )
  return (
    <QueryClientProvider client={queryClient}>
      <LiveUpdates />
      {children}
    </QueryClientProvider>
  )
}
