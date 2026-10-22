import { useSyncExternalStore } from 'react'

const storageKey = 'ariane.scope.descendants'
const listeners = new Set<() => void>()
// Kept in memory too, for a browser that refuses storage: the toggle then lasts until the page reloads.
let current = read()

function read(): boolean {
  try {
    return localStorage.getItem(storageKey) === '1'
  } catch {
    return false
  }
}

export function setIncludeDescendants(include: boolean) {
  current = include
  try {
    localStorage.setItem(storageKey, include ? '1' : '0')
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

// Whether a node in scope brings the items of its sub-nodes along; off by default, remembered by the
// browser, shared by every view.
export function useIncludeDescendants(): boolean {
  return useSyncExternalStore(subscribe, () => current, () => false)
}
