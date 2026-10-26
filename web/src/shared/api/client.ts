import type {
  CommitResult,
  Dashboard,
  DiscardResult,
  PullResult,
  FileDiff,
  Finding,
  Item,
  ItemDetail,
  ItemPatch,
  Kind,
  NodeSummary,
  Repo,
  Schema,
  SearchHit,
  Directory,
  MondayUser,
  PersonChange,
  PersonaChange,
  PersonaDirectory,
  ProxyReport,
  RouteChange,
  RouteTable,
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
export const fetchSearch = (q: string, archived: boolean) =>
  request<SearchHit[]>('GET', `/api/search?${new URLSearchParams(archived ? { q, archived: '1' } : { q }).toString()}`)
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
export const fetchPersonas = () => request<PersonaDirectory[]>('GET', '/api/personas')
export const addPersona = (persona: PersonaChange) => request<PersonaDirectory[]>('POST', '/api/personas', persona)
export const changePersona = (persona: PersonaChange) => request<PersonaDirectory[]>('PATCH', '/api/personas', persona)
export const removePersona = (persona: Pick<PersonaChange, 'node' | 'id'>) => request<PersonaDirectory[]>('DELETE', '/api/personas', persona)
export const findMondayUsers = (account: string, q: string) =>
  request<MondayUser[]>('GET', `/api/monday/users?${new URLSearchParams({ account, q }).toString()}`)
export const syncMonday = (path: string) => request<SyncReport>('POST', '/api/sync/monday', { path })
export const fetchSettings = () => request<Settings>('GET', '/api/settings')
export const saveTheme = (theme: Theme) => request<Settings>('PUT', '/api/settings/theme', theme)
export const commitChanges = (message: string) => request<CommitResult>('POST', '/api/commit', { message })
export const pushChanges = () => request<{ ok: boolean }>('POST', '/api/push', {})
export const pullChanges = () => request<PullResult>('POST', '/api/pull', {})
export const discardChanges = (paths?: string[]) =>
  request<DiscardResult>('POST', '/api/discard', paths === undefined ? {} : { paths })
export const fetchProxy = () => request<ProxyReport>('GET', '/api/proxy')
export const setRoute = (route: RouteChange) => request<RouteTable>('PUT', '/api/proxy/routes', route)
export const removeRoute = (node: string) => request<RouteTable>('DELETE', '/api/proxy/routes', { node })
