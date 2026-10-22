export interface Group<T> {
  key: string
  label: string
  items: T[]
}

export function compareNatural(left: string, right: string): number {
  return left.localeCompare(right, undefined, { numeric: true, sensitivity: 'base' })
}

// Groups items by the key `keyOf` gives, groups in natural order (`0.9` before `0.10`), the items
// without a key in one last group labelled `none`.
export function groupBy<T>(items: T[], keyOf: (item: T) => string | null, none: string, label = (key: string) => key): Group<T>[] {
  const groups = new Map<string, T[]>()
  const missing: T[] = []
  for (const item of items) {
    const key = keyOf(item)
    if (key === null || key === '') {
      missing.push(item)
    } else {
      groups.set(key, [...(groups.get(key) ?? []), item])
    }
  }
  const sorted = [...groups.entries()]
    .sort(([left], [right]) => compareNatural(left, right))
    .map(([key, entries]) => ({ key, label: label(key), items: entries }))
  return missing.length === 0 ? sorted : [...sorted, { key: '', label: none, items: missing }]
}
