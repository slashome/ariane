export const searchParamKeys = {
  node: 'node',
  query: 'q',
  kinds: 'kind',
  categories: 'category',
  archived: 'archived',
} as const

// The part of a query string that follows the user from view to view: the node scope and the archive
// toggle. Filters proper to one view (search, kind, status) stay behind.
export function scopeSearch(search: string): string {
  const current = new URLSearchParams(search)
  const kept = new URLSearchParams()
  for (const key of [searchParamKeys.node, searchParamKeys.archived]) {
    const value = current.get(key)
    if (value !== null) {
      kept.set(key, value)
    }
  }
  const text = kept.toString()
  return text === '' ? '' : `?${text}`
}
