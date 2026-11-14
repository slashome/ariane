import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { makeItem, type NodeSummary, type Release } from '@/shared/api'
import type { DragEndEvent } from '@/shared/ui'
import { makeRoadmap } from '../model/fixtures'
import { ReleasesPage } from './releases-page'

const captured: { onDragEnd: ((event: DragEndEvent) => void) | null } = { onDragEnd: null }

vi.mock('@/shared/ui', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/shared/ui')>()
  return {
    ...actual,
    DragArea: ({ onDragEnd, children }: { onDragEnd: (event: DragEndEvent) => void; children: React.ReactNode }) => {
      captured.onDragEnd = onDragEnd
      return children
    },
  }
})

const releases: Release[] = [
  { name: '0.2', parking: false, target: '2027-01-15' },
  { name: 'plus-tard', parking: true, target: null },
]

const nodes: NodeSummary[] = [
  { path: 'p', items: 3, workflow: [], releases, releases_from: 'p' },
  { path: 'p/c', items: 1, workflow: [], releases, releases_from: 'p' },
]

const items = [
  makeItem({ id: 'T1', title: 'Loose task', type: 'bug', path: 'tree/p/tasks/T1.md' }),
  makeItem({
    id: 'T2',
    title: 'Planned task',
    release: '0.2',
    effective_release: { name: '0.2', source: 'own', from: null, position: 0, parking: false },
    path: 'tree/p/tasks/T2.md',
    reference: 'p#T2',
  }),
  makeItem({ id: 'S1', kind: 'story', title: 'A story', path: 'tree/p/stories/S1.md' }),
  makeItem({ id: 'T3', node: 'p/c', title: 'Child task', path: 'tree/p/c/tasks/T3.md' }),
]

interface Call {
  method: string
  url: string
  body: unknown
}

function renderAt(url: string) {
  const calls: Call[] = []
  vi.stubGlobal(
    'fetch',
    vi.fn(async (input: string, init?: RequestInit) => {
      const method = init?.method ?? 'GET'
      calls.push({ method, url: input, body: init?.body ? JSON.parse(String(init.body)) : null })
      const body =
        method === 'PATCH' && input === '/api/items'
          ? items[0]
          : input.startsWith('/api/nodes')
            ? nodes
            : input.startsWith('/api/items')
              ? items
              : input.startsWith('/api/roadmap')
                ? makeRoadmap([])
                : {}
      return new Response(JSON.stringify(body), { headers: { 'Content-Type': 'application/json' } })
    }),
  )
  const router = createMemoryRouter([{ path: '/roadmap/releases', element: <ReleasesPage /> }], { initialEntries: [url] })
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  )
  return { calls, router }
}

afterEach(() => {
  vi.unstubAllGlobals()
  captured.onDragEnd = null
})

function drop(path: string, zone: string) {
  act(() => captured.onDragEnd?.({ active: { id: path }, over: { id: zone } } as unknown as DragEndEvent))
}

describe('the Releases page', () => {
  it('draws one column per release, parking last, and the unplanned work in the backlog', async () => {
    renderAt('/roadmap/releases?node=p')
    const column = await screen.findByLabelText('Release 0.2')
    expect(within(column).getByText('Planned task')).toBeInTheDocument()
    expect(screen.getByLabelText('Target of 0.2')).toHaveValue('2027-01-15')
    const backlog = screen.getByLabelText('Backlog')
    expect(within(backlog).getByText('Loose task')).toBeInTheDocument()
    expect(within(backlog).getByText('Child task')).toBeInTheDocument()
    expect(within(backlog).queryByText('Planned task')).toBeNull()
    await userEvent.selectOptions(within(backlog).getByLabelText('Type'), 'bug')
    expect(within(backlog).queryByText('A story')).toBeNull()
    expect(within(backlog).getByText('Loose task')).toBeInTheDocument()
    const headings = screen.getAllByRole('heading', { level: 2 }).map((heading) => heading.textContent)
    expect(headings).toEqual(['Backlog', '0.2', 'plus-tard'])
  })

  it('plans a dropped card in its column and takes it back out on the backlog', async () => {
    const { calls } = renderAt('/roadmap/releases?node=p')
    await screen.findByLabelText('Release 0.2')
    drop('tree/p/tasks/T2.md', 'release:0.2')
    drop('tree/p/tasks/T1.md', 'release:plus-tard')
    drop('tree/p/tasks/T2.md', 'backlog')
    await vi.waitFor(() => expect(calls.filter((call) => call.method === 'PATCH')).toHaveLength(2))
    expect(calls.filter((call) => call.method === 'PATCH').map((call) => call.body)).toEqual([
      { path: 'tree/p/tasks/T1.md', changes: { release: { from: null, to: 'plus-tard' } } },
      { path: 'tree/p/tasks/T2.md', changes: { release: { from: '0.2', to: null } } },
    ])
  })

  it('sends a new target date and a parking toggle to the release list', async () => {
    const { calls } = renderAt('/roadmap/releases?node=p')
    const target = await screen.findByLabelText('Target of 0.2')
    await userEvent.clear(target)
    await userEvent.tab()
    await userEvent.click(screen.getByRole('button', { name: 'Release 0.2 actions' }))
    await userEvent.click(await screen.findByRole('menuitem', { name: 'Make it a parking release' }))
    await vi.waitFor(() => expect(calls.filter((call) => call.url === '/api/releases')).toHaveLength(2))
    expect(calls.filter((call) => call.url === '/api/releases').map((call) => [call.method, call.body])).toEqual([
      ['PATCH', { node: 'p', name: '0.2', target: null }],
      ['PATCH', { node: 'p', name: '0.2', parking: true }],
    ])
  })

  it('keeps an inherited list read-only, names its node and still plans items', async () => {
    const { calls, router } = renderAt('/roadmap/releases?node=p/c')
    const notice = await screen.findByRole('note', { name: 'Inherited releases' })
    expect(screen.getByRole('button', { name: /New release/ })).toBeDisabled()
    expect(screen.getByLabelText('Target of 0.2')).toBeDisabled()
    drop('tree/p/c/tasks/T3.md', 'release:0.2')
    await vi.waitFor(() => expect(calls.some((call) => call.method === 'PATCH' && call.url === '/api/items')).toBe(true))
    await userEvent.click(within(notice).getByRole('link', { name: 'Open p' }))
    expect(router.state.location.search).toBe('?node=p')
  })
})
