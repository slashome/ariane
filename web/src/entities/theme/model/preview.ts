import { useSyncExternalStore } from 'react'
import type { Theme } from '@/shared/api'

// A theme being tried in Settings and not saved yet: shown everywhere while it lasts.
let current: Theme | null = null
const listeners = new Set<() => void>()

export function previewTheme(theme: Theme | null) {
  current = theme
  listeners.forEach((listener) => listener())
}

function subscribe(listener: () => void) {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

export function useThemePreview(): Theme | null {
  return useSyncExternalStore(subscribe, () => current, () => null)
}
