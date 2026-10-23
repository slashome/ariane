import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { makeItem, type NodeSummary } from '@/shared/api'
import { KanbanPage } from './kanban-page'

const nodes: NodeSummary[] = [
  {
    path: 'p',
    has_index: true,
    items: 3,
    workflow: [
      { id: 'todo', label: 'To do', category: 'todo' },
      { id: 'waiting', label: 'Waiting', category: 'waiting' },
      { id: 'done', label: 'Done', category: 'done' },
    ],
    people: { keyvan: { name: 'Keyvan Coen' } },
  },
]

const items = [
  makeItem({ id: 'T1', title: 'First' }),
  makeItem({ id: 'T2', title: 'Second' }),
  makeItem({ id: 'T3', title: 'Third', status: 'waiting', category: 'waiting', waiting_for: ['keyvan'], waiting_since: '2026-10-20' }),
]

function renderAt(url: string) {
  vi.stubGlobal(
    'fetch',
    vi.fn(async (input: string) => {
      const body = input.startsWith('/api/nodes') ? nodes : input.startsWith('/api/items') ? items : {}
      return new Response(JSON.stringify(body), { headers: { 'Content-Type': 'application/json' } })
    }),
  )
  const router = createMemoryRouter([{ path: '/tasks/kanban', element: <KanbanPage /> }], { initialEntries: [url] })
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  )
  return router
}

afterEach(() => {
  vi.unstubAllGlobals()
  localStorage.clear()
})

describe('KanbanPage', () => {
  it('shows who a waiting card waits for', async () => {
    renderAt('/tasks/kanban?node=p')
    expect(await screen.findByText(/Keyvan Coen · \d+ d/)).toBeInTheDocument()
  })

  it('keeps the preview in the URL and steps through the cards', async () => {
    const router = renderAt('/tasks/kanban?node=p')
    await userEvent.click(await screen.findByRole('button', { name: 'Open First' }))
    expect(router.state.location.search).toContain('item=tree%2Fp%2Ftasks%2FT1.md')
    const sheet = await screen.findByRole('dialog')
    expect(within(sheet).getByText('1 / 3')).toBeInTheDocument()
    expect(within(sheet).getByRole('button', { name: 'Previous item' })).toBeDisabled()
    await userEvent.click(within(sheet).getByRole('button', { name: 'Next item' }))
    expect(router.state.location.search).toContain('T2.md')
    expect(within(await screen.findByRole('dialog')).getByText('2 / 3')).toBeInTheDocument()
    await router.navigate(-1)
    expect(router.state.location.search).toContain('T1.md')
  })

  it('remembers a folded column', async () => {
    renderAt('/tasks/kanban?node=p')
    await userEvent.click(await screen.findByRole('button', { name: /To do/ }))
    expect(JSON.parse(localStorage.getItem('ariane.kanban.toggled') ?? '[]')).toEqual(['status:todo'])
  })
})
