import type {
  CommitResult,
  Dashboard,
  DiscardResult,
  FileDiff,
  Finding,
  Item,
  ItemDetail,
  ItemPatch,
  Kind,
  NodeSummary,
  Repo,
  Schema,
  Directory,
  MondayUser,
  PersonChange,
  Settings,
  SyncReport,
  Theme,
} from './types'

export class ApiError extends Error {
  constructor(
    readonly url: string,
    readonly status: number,
    readonly body: unknown = null,
  ) {
    super(messageOf(body) ?? `${url} answered ${status}`)
  }
}

function messageOf(body: unknown): string | null {
  if (typeof body === 'object' && body !== null && 'error' in body && typeof body.error === 'string') {
    return body.error
  }
  return null
}

async function readBody(response: Response): Promise<unknown> {
  const text = await response.text()
  if (text === '') {
    return null
  }
  try {
    return JSON.parse(text) as unknown
  } catch {
    return text
  }
}

async function request<T>(method: string, url: string, payload?: unknown): Promise<T> {
  const response = await fetch(url, {
    method,
    headers:
      payload === undefined
        ? { Accept: 'application/json' }
        : { Accept: 'application/json', 'Content-Type': 'application/json' },
    body: payload === undefined ? undefined : JSON.stringify(payload),
  })
  const body = await readBody(response)
  if (!response.ok) {
    throw new ApiError(url, response.status, body)
  }
  return body as T
}

export const fetchItems = () => request<Item[]>('GET', '/api/items')
export const fetchItemsWithBody = (kind: Kind) =>
  request<Item[]>('GET', `/api/items?${new URLSearchParams({ kind, body: '1' }).toString()}`)
export const fetchItem = (reference: string) =>
  request<ItemDetail>('GET', `/api/show?${new URLSearchParams({ ref: reference }).toString()}`)
export const fetchDoctor = () => request<Finding[]>('GET', '/api/doctor')
export const fetchNodes = () => request<NodeSummary[]>('GET', '/api/nodes')
export const fetchSchema = () => request<Schema>('GET', '/api/schema')
export const fetchRepo = () => request<Repo>('GET', '/api/repo')
export const fetchDashboard = () => request<Dashboard>('GET', '/api/dashboard')
export const fetchDiff = (path: string) =>
  request<FileDiff>('GET', `/api/diff?${new URLSearchParams({ path }).toString()}`)

export const patchItem = (patch: ItemPatch) => request<Item>('PATCH', '/api/items', patch)
export const saveDashboard = (dashboard: Dashboard) => request<Dashboard>('PUT', '/api/dashboard', dashboard)
export const fetchPeople = () => request<Directory[]>('GET', '/api/people')
export const addPerson = (person: PersonChange) => request<Directory[]>('POST', '/api/people', person)
export const changePerson = (person: PersonChange) => request<Directory[]>('PATCH', '/api/people', person)
export const removePerson = (person: Pick<PersonChange, 'node' | 'handle'>) => request<Directory[]>('DELETE', '/api/people', person)
export const findMondayUsers = (account: string, q: string) =>
  request<MondayUser[]>('GET', `/api/monday/users?${new URLSearchParams({ account, q }).toString()}`)
export const syncMonday = (path: string) => request<SyncReport>('POST', '/api/sync/monday', { path })
export const fetchSettings = () => request<Settings>('GET', '/api/settings')
export const saveTheme = (theme: Theme) => request<Settings>('PUT', '/api/settings/theme', theme)
export const commitChanges = (message: string) => request<CommitResult>('POST', '/api/commit', { message })
export const pushChanges = () => request<{ ok: boolean }>('POST', '/api/push', {})
export const discardChanges = (paths?: string[]) =>
  request<DiscardResult>('POST', '/api/discard', paths === undefined ? {} : { paths })
