import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act, renderHook, waitFor } from '@testing-library/react'
import type { ReactNode } from 'react'
import { queryKeys, type Item } from '@/shared/api'
import { changesFor } from '../model/categories'
import { useUpdateItem } from './use-update-item'

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
  created: null,
  external: [],
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
})

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
