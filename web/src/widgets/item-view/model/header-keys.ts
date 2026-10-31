// Header keys the item view already shows in its own fields; any other key is a custom key of the node.
const shown = new Set([
  'ariane',
  'node',
  'id',
  'title',
  'status',
  'type',
  'parent',
  'depends_on',
  'size',
  'impact',
  'priority',
  'rank',
  'created',
  'release',
  'milestone',
  'refs',
  'external',
  'branches',
  'pull_requests',
  'waiting_for',
  'waiting_on',
  'waiting_since',
])

export function customEntries(header: Record<string, unknown> | null): [string, string][] {
  return Object.entries(header ?? {})
    .filter(([key]) => !shown.has(key))
    .map(([key, value]) => [key, Array.isArray(value) ? value.join(', ') : typeof value === 'object' ? JSON.stringify(value) : String(value)])
}

export function refsOf(header: Record<string, unknown> | null): string[] {
  const refs = header?.refs
  return Array.isArray(refs) ? refs.map(String) : []
}
