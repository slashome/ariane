import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useState, type ReactNode } from 'react'
import { Toaster } from '@/shared/ui'
import { AppearanceSync } from './appearance-sync'
import { LiveUpdates } from './live-updates'
import { ThemeSync } from './theme-sync'

export function AppProviders({ children }: { children: ReactNode }) {
  const [queryClient] = useState(
    () => new QueryClient({ defaultOptions: { queries: { staleTime: 30_000, refetchOnWindowFocus: false } } }),
  )
  return (
    <QueryClientProvider client={queryClient}>
      <LiveUpdates />
      <ThemeSync />
      <AppearanceSync />
      {children}
      <Toaster />
    </QueryClientProvider>
  )
}
