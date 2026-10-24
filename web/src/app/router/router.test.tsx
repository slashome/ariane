import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { makeItem, type Schema } from '@/shared/api'
import { routeTree } from './router'

const schema: Schema = {
  version: 1,
  categories: [
    { id: 'backlog', label: 'Backlog' },
    { id: 'todo', label: 'To do' },
    { id: 'in_progress', label: 'In progress' },
    { id: 'blocked', label: 'Blocked' },
    { id: 'done', label: 'Done' },
    { id: 'dropped', label: 'Dropped' },
  ],
  id_prefixes: { reflexion: 'R', ressource: 'D', story: 'S', task: 'T', unknown: 'T' },
  keys: [
    { name: 'priority', type: 'enum', values: ['low', 'medium', 'high'] },
    { name: 'impact', type: 'enum', values: ['low', 'medium', 'high'] },
    { name: 'size', type: 'enum', values: ['XS', 'S', 'M', 'L', 'XL'] },
  ],
  reflexion_statuses: ['open', 'trial', 'settled'],
}

const items = [
  makeItem({ id: 'S1', kind: 'story', title: 'Live story', path: 'tree/p/stories/S1.md' }),
  makeItem({ id: 'S2', kind: 'story', title: 'Old story', archived: true, category: 'done', status: 'done', path: 'tree/p/archives/stories/S2.md' }),
  makeItem({ id: 'T1', title: 'Child task', parent: 'p#S1', category: 'done', status: 'done' }),
  makeItem({ id: 'T2', title: 'Waiting task', category: 'backlog', status: 'backlog' }),
  makeItem({ id: 'T3', title: 'Archived task', archived: true, category: 'done', status: 'done', path: 'tree/p/archives/tasks/T3.md' }),
]

let proxy: unknown = {}

function answer(url: string): unknown {
  if (url.startsWith('/api/schema')) return schema
  if (url.startsWith('/api/nodes')) return [{ path: 'p', items: 5, workflow: [], tracker: null }]
  if (url.startsWith('/api/items')) return items
  if (url.startsWith('/api/doctor')) return []
  if (url.startsWith('/api/dashboard')) return { pinned: ['p'] }
  if (url.startsWith('/api/proxy')) return proxy
  if (url.startsWith('/api/repo')) return { root: '/r', branch: 'main', ahead: 0, uncommitted: [{ code: 'M', path: 'tree/p/tasks/T1.md' }] }
  return {}
}

function renderAt(url: string) {
  vi.stubGlobal(
    'fetch',
    vi.fn(async (input: string) => new Response(JSON.stringify(answer(input)), { headers: { 'Content-Type': 'application/json' } })),
  )
  const router = createMemoryRouter(routeTree, { initialEntries: [url] })
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(
    <QueryClientProvider client={client}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  )
  return router
}

afterEach(() => {
  proxy = {}
  vi.unstubAllGlobals()
  localStorage.clear()
})

describe('the navigation', () => {
  it('sends the former addresses to their new place, filters kept', async () => {
    const router = renderAt('/kanban?node=p')
    await waitFor(() => expect(router.state.location.pathname).toBe('/tasks/kanban'))
    expect(router.state.location.search).toBe('?node=p')
  })

  it('counts each menu entry and the status bar shows the branch and the uncommitted files', async () => {
    renderAt('/tasks/backlog')
    const nav = screen.getByRole('navigation', { name: 'Main' })
    expect(await within(nav).findByRole('link', { name: 'Backlog, 1' })).toBeInTheDocument()
    expect(within(nav).getByRole('link', { name: 'Stories, 1' })).toBeInTheDocument()
    const status = screen.getByRole('contentinfo', { name: 'Status' })
    expect(within(status).getByText('main')).toBeInTheDocument()
    expect(within(status).getByText('1 uncommitted')).toBeInTheDocument()
    expect(await screen.findByText('Waiting task')).toBeInTheDocument()
  })

  it('brings archived items into the views and the counters with the status bar toggle', async () => {
    const router = renderAt('/stories')
    expect(await screen.findByText('Live story')).toBeInTheDocument()
    expect(screen.queryByText('Old story')).not.toBeInTheDocument()
    await userEvent.click(screen.getByRole('switch', { name: 'Include archived items' }))
    expect(await screen.findByText('Old story')).toBeInTheDocument()
    expect(router.state.location.search).toBe('?archived=1')
    const nav = screen.getByRole('navigation', { name: 'Main' })
    expect(within(nav).getByRole('link', { name: 'Stories, 2' })).toBeInTheDocument()
    expect(within(nav).getByRole('link', { name: 'List, 3' })).toBeInTheDocument()
  })

  it('folds a menu section from the keyboard', async () => {
    renderAt('/')
    const toggle = await screen.findByRole('button', { name: 'Tasks' })
    toggle.focus()
    await userEvent.keyboard('{Enter}')
    expect(toggle).toHaveAttribute('aria-expanded', 'false')
    expect(screen.queryByRole('link', { name: /^Kanban/ })).not.toBeInTheDocument()
  })

  it('puts the proxy between Repository and People, with its route count and a dot for the chain', async () => {
    proxy = {
      ok: false,
      proxy_port: 27380,
      server_port: 27317,
      checks: [{ ok: false, what: 'alias 127.0.0.2 on lo0', hint: 'missing: run `ariane proxy setup`' }],
      routes: [
        { host: 'ariane.local', port: 27317, node: null, declared: false, listening: true },
        { host: 'p.local', port: 3000, node: 'p', declared: false, listening: false },
      ],
      problems: [],
    }
    renderAt('/proxy')
    const system = screen.getByRole('navigation', { name: 'System' })
    const entry = await within(system).findByRole('link', { name: 'Proxy, 2, a link is broken' })
    expect(entry).toHaveAttribute('href', '/proxy')
    expect(within(entry).getByTestId('status-dot')).toHaveClass('bg-red-500')
    const labels = within(system)
      .getAllByRole('link')
      .map((link) => link.textContent?.replace(/\d+$/, ''))
    expect(labels).toEqual(['Health', 'Repository', 'Proxy', 'People', 'Settings'])
    expect(await screen.findByRole('heading', { name: 'Proxy', level: 1 })).toBeInTheDocument()
  })
})
