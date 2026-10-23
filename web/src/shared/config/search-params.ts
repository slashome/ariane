export const searchParamKeys = {
  node: 'node',
  query: 'q',
  kinds: 'kind',
  categories: 'category',
  archived: 'archived',
  // The item previewed over any view, opened from the command palette.
  preview: 'preview',
} as const

// The part of a query string that follows the user from view to view: the node scope, the archive
// toggle and the search field. Filters proper to one view (kind, status) stay behind.
export function scopeSearch(search: string): string {
  const current = new URLSearchParams(search)
  const kept = new URLSearchParams()
  for (const key of [searchParamKeys.node, searchParamKeys.archived, searchParamKeys.query]) {
    const value = current.get(key)
    if (value !== null) {
      kept.set(key, value)
    }
  }
  const text = kept.toString()
  return text === '' ? '' : `?${text}`
}
