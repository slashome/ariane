import { useSyncExternalStore } from 'react'

export type Appearance = 'system' | 'light' | 'dark'

export const appearances: Appearance[] = ['system', 'light', 'dark']

const storageKey = 'ariane.appearance'
const listeners = new Set<() => void>()
// Kept in memory too, for a browser that refuses storage: the choice then lasts until the page reloads.
let current = readAppearance()

export function readAppearance(): Appearance {
  try {
    const stored = localStorage.getItem(storageKey)
    return appearances.find((appearance) => appearance === stored) ?? 'system'
  } catch {
    return 'system'
  }
}

export function setAppearance(appearance: Appearance) {
  current = appearance
  try {
    localStorage.setItem(storageKey, appearance)
  } catch {
    // Remembered for this page only.
  }
  listeners.forEach((listener) => listener())
}

function subscribe(listener: () => void) {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

// Light or dark, as chosen in this browser, or as the system says; the system by default.
export function useAppearance(): Appearance {
  return useSyncExternalStore(subscribe, () => current, () => 'system' as const)
}

export function resolveAppearance(appearance: Appearance, systemDark: boolean): 'light' | 'dark' {
  return appearance === 'system' ? (systemDark ? 'dark' : 'light') : appearance
}
