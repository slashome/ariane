import type { Item, NodeSummary, Repo, Schema } from './types'

export class ApiError extends Error {
  constructor(
    readonly url: string,
    readonly status: number,
  ) {
    super(`GET ${url} answered ${status}`)
  }
}

async function getJson<T>(url: string): Promise<T> {
  const response = await fetch(url, { headers: { Accept: 'application/json' } })
  if (!response.ok) {
    throw new ApiError(url, response.status)
  }
  return (await response.json()) as T
}

export const fetchItems = () => getJson<Item[]>('/api/items')
export const fetchNodes = () => getJson<NodeSummary[]>('/api/nodes')
export const fetchSchema = () => getJson<Schema>('/api/schema')
export const fetchRepo = () => getJson<Repo>('/api/repo')
