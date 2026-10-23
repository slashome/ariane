import { useCallback } from 'react'
import { useSearchParams } from 'react-router'

// One query parameter as state. Setting it pushes a history entry, so the browser's Back returns to
// the previous value: an item preview opened from a list closes on Back, and comes back on Back from
// the item's page.
export function useSearchParam(key: string): [string | null, (value: string | null) => void] {
  const [params, setParams] = useSearchParams()
  const set = useCallback(
    (value: string | null) =>
      setParams((current) => {
        const next = new URLSearchParams(current)
        if (value === null) {
          next.delete(key)
        } else {
          next.set(key, value)
        }
        return next
      }),
    [key, setParams],
  )
  return [params.get(key), set]
}
