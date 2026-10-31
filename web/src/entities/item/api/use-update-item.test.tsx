import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act, renderHook, waitFor } from '@testing-library/react'
import type { ReactNode } from 'react'
import { queryKeys, type Item, type NodeSummary } from '@/shared/api'
import { toast } from '@/shared/ui'
import { changesFor } from '../model/categories'
import { useUpdateItem } from './use-update-item'

vi.mock('@/shared/ui', () => ({ toast: { info: vi.fn(), success: vi.fn(), warning: vi.fn(), error: vi.fn() } }))

const item: Item = {
  node: 'game',
  id: 'T001',
  title: 'Forge',
  kind: 'task',
  archived: false,
  status: 'a-faire',
  category: 'todo',
  size: 'M',
  impact: null,
  priority: null,
  rank: null,
  created: null,
  reference: null,
  type: null,
  personas: [],
  release: null,
  milestone: null,
  parent: null,
  epic: null,
  depends_on: [],
  external: [],
  branches: [],
  pull_requests: [],
  waiting_for: [],
  waiting_on: null,
  waiting_since: null,
  version: 1,
  path: 'tree/game/tasks/T001.md',
}

function setup(respond: (body: unknown) => Response) {
  const calls: { method: string; body: unknown }[] = []
  vi.stubGlobal(
    'fetch',
    vi.fn(async (_url: string, init: RequestInit) => {
      const body = JSON.parse(String(init.body)) as unknown
      calls.push({ method: init.method ?? 'GET', body })
      return respond(body)
    }),
  )
  const client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } })
  client.setQueryData(queryKeys.items, [item])
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  )
  const { result } = renderHook(() => useUpdateItem(), { wrapper })
  return { calls, client, result }
}

afterEach(() => {
  vi.unstubAllGlobals()
  vi.clearAllMocks()
})

function moveWithNode(tracker: NodeSummary['tracker']) {
  const saved = { ...item, status: 'a-tester', category: 'in_progress' as const }
  const { client, result } = setup(() => Response.json(saved))
  client.setQueryData<NodeSummary[]>(queryKeys.nodes, [{ path: 'game', items: 1, workflow: [], tracker }])
  act(() => {
    result.current.mutate({
      item,
      changes: changesFor(item, { status: 'a-tester' }),
      optimistic: { status: 'a-tester', category: 'in_progress' },
    })
  })
  return result
}

describe('useUpdateItem', () => {
  it('sends the current value as `from` and keeps the server row', async () => {
    const saved = { ...item, status: 'a-tester', category: 'in_progress' as const }
    const { calls, client, result } = setup(() => Response.json(saved))
    act(() => {
      result.current.mutate({
        item,
        changes: changesFor(item, { status: 'a-tester' }),
        optimistic: { status: 'a-tester', category: 'in_progress' },
      })
    })
    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(calls).toEqual([
      { method: 'PATCH', body: { path: item.path, changes: { status: { from: 'a-faire', to: 'a-tester' } } } },
    ])
    expect(client.getQueryData<Item[]>(queryKeys.items)?.[0]?.status).toBe('a-tester')
  })

  it('says so when an item of a node bound to monday has no ticket', async () => {
    const result = moveWithNode({ kind: 'monday', account: 'acme', board: 7 })
    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(toast.info).toHaveBeenCalledWith('tree/game/tasks/T001.md: no monday ticket linked, monday not updated')
  })

  it('stays quiet on a node without a monday tracker', async () => {
    const result = moveWithNode(null)
    await waitFor(() => expect(result.current.isSuccess).toBe(true))
    expect(toast.info).not.toHaveBeenCalled()
  })

  it('rolls the optimistic change back when the file changed on disk', async () => {
    const { client, result } = setup(() => Response.json({ error: 'stale', current: item }, { status: 409 }))
    act(() => {
      result.current.mutate({
        item,
        changes: changesFor(item, { size: 'XS' }),
        optimistic: { size: 'XS' },
      })
    })
    await waitFor(() => expect(result.current.isError).toBe(true))
    expect(client.getQueryData<Item[]>(queryKeys.items)?.[0]?.size).toBe('M')
  })
})
