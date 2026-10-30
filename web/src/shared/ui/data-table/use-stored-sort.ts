import { useCallback, useMemo, useSyncExternalStore } from 'react'
import { parseSort, type TableSort } from './sort'

const fallback = new Map<string, string | null>()
const listeners = new Set<() => void>()

function storageKey(table: string): string {
  return `ariane.sort.${table}`
}

function read(key: string): string | null {
  try {
    return localStorage.getItem(key)
  } catch {
    return fallback.get(key) ?? null
  }
}

function persist(key: string, raw: string | null): boolean {
  try {
    if (raw === null) {
      localStorage.removeItem(key)
    } else {
      localStorage.setItem(key, raw)
    }
    return true
  } catch {
    return false
  }
}

function write(key: string, raw: string | null) {
  if (!persist(key, raw)) {
    fallback.set(key, raw)
  }
  listeners.forEach((listener) => listener())
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

export function useStoredSort(table: string): [TableSort | null, (sort: TableSort | null) => void] {
  const key = storageKey(table)
  const raw = useSyncExternalStore(subscribe, () => read(key))
  const sort = useMemo(() => parseSort(raw), [raw])
  const setSort = useCallback((next: TableSort | null) => write(key, next === null ? null : JSON.stringify(next)), [key])
  return [sort, setSort]
}
