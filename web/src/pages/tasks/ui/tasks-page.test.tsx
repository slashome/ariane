import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider } from 'react-router'
import type { Item, Schema } from '@/shared/api'
import { TasksPage } from './tasks-page'

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
  keys: [{ name: 'size', type: 'enum', values: ['XS', 'S', 'M', 'L', 'XL'] }],
  reflexion_statuses: ['open', 'trial', 'settled'],
}

const base: Item = {
  node: 'acme/web',
  id: 'T001',
  title: 'Paint the fence',
  kind: 'task',
  archived: false,
  status: 'todo',
  category: 'todo',
  size: 'XL',
  impact: null,
  priority: null,
  created: '2030-01-02',
  reference: null,
  type: null,
  release: null,
  milestone: null,
  parent: null,
  depends_on: [],
  external: [],
  version: 1,
  path: 'tree/acme/web/tasks/T001.md',
}

const items: Item[] = [
  base,
  { ...base, id: 'T002', title: 'Fix the gate', size: 'XS', status: 'blocked', category: 'blocked', path: 'tree/acme/web/tasks/T002.md' },
  { ...base, node: 'acme/api', id: 'T003', title: 'Archived thing', archived: true, path: 'tree/acme/api/archives/tasks/T003.md' },
]

function renderPage(url: string) {
  vi.stubGlobal(
    'fetch',
    vi.fn(async (input: string) => {
      const body = input.endsWith('/api/schema') ? schema : input.endsWith('/api/nodes') ? [] : items
      return new Response(JSON.stringify(body), { headers: { 'Content-Type': 'application/json' } })
    }),
  )
  const router = createMemoryRouter([{ path: '/tasks', element: <TasksPage /> }], { initialEntries: [url] })
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(
    <QueryClientProvider client={client}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  )
}

const titles = () =>
  screen
    .getAllByRole('row')
    .slice(1)
    .map((row) => within(row).getAllByRole('cell')[2]?.textContent)

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('TasksPage', () => {
  it('lists live items and counts them against the total', async () => {
    renderPage('/tasks')
    expect(await screen.findByText('Paint the fence')).toBeInTheDocument()
    expect(screen.getByText('Fix the gate')).toBeInTheDocument()
    expect(screen.queryByText('Archived thing')).not.toBeInTheDocument()
    expect(screen.getByText('2 of 3 tasks')).toBeInTheDocument()
  })

  it('sorts sizes by their order in the schema, not alphabetically', async () => {
    renderPage('/tasks')
    await screen.findByText('Paint the fence')
    await userEvent.click(screen.getByRole('button', { name: /size/i }))
    expect(titles()).toEqual(['Fix the gate', 'Paint the fence'])
  })

  it('applies the filters carried by the URL', async () => {
    renderPage('/tasks?category=blocked&archived=1')
    expect(await screen.findByText('Fix the gate')).toBeInTheDocument()
    expect(screen.queryByText('Paint the fence')).not.toBeInTheDocument()
    expect(screen.getByText('1 of 3 tasks')).toBeInTheDocument()
  })
})
