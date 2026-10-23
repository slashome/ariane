import { useCallback, useState } from 'react'

function read(key: string): Set<string> {
  try {
    const stored: unknown = JSON.parse(localStorage.getItem(key) ?? '[]')
    return new Set(Array.isArray(stored) ? stored.filter((entry): entry is string => typeof entry === 'string') : [])
  } catch {
    return new Set()
  }
}

// A set of strings the browser remembers under `key`, such as the folded columns of a board. Storage
// that is refused or corrupt only forgets: the set starts empty.
export function useStoredSet(key: string): [Set<string>, (value: string) => void] {
  const [values, setValues] = useState(() => read(key))
  const toggle = useCallback(
    (value: string) =>
      setValues((current) => {
        const next = new Set(current)
        if (next.has(value)) {
          next.delete(value)
        } else {
          next.add(value)
        }
        try {
          localStorage.setItem(key, JSON.stringify([...next]))
        } catch {
          // Remembered for this page only.
        }
        return next
      }),
    [key],
  )
  return [values, toggle]
}
