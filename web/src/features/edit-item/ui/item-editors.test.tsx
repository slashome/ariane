import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { makeItem, type NodeSummary, type Schema } from '@/shared/api'
import { ItemEditors } from './item-editors'

const nodes: NodeSummary[] = [
  {
    path: 'p',
    items: 1,
    workflow: [
      { id: 'todo', label: 'To do', category: 'todo' },
      { id: 'waiting', label: 'Waiting', category: 'waiting' },
    ],
    people: { keyvan: { name: 'Keyvan Coen' } },
  },
]

const schema = {
  version: 1,
  categories: [],
  id_prefixes: {},
  keys: [
    { name: 'priority', type: 'enum', values: ['low', 'medium', 'high'] },
    { name: 'size', type: 'enum', values: ['XS', 'S', 'M'] },
    { name: 'impact', type: 'enum', values: ['low', 'medium', 'high'] },
  ],
  reflexion_statuses: ['open', 'trial', 'settled'],
} as unknown as Schema

function renderEditors(item = makeItem()) {
  const fetch = vi.fn(async (url: string, init?: RequestInit) => {
    const body = url.startsWith('/api/nodes') ? nodes : url.startsWith('/api/schema') ? schema : init?.method === 'PATCH' ? item : []
    return new Response(JSON.stringify(body), { headers: { 'Content-Type': 'application/json' } })
  })
  vi.stubGlobal('fetch', fetch)
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <ItemEditors item={item} />
    </QueryClientProvider>,
  )
  return fetch
}

const patches = (fetch: ReturnType<typeof vi.fn>) =>
  fetch.mock.calls.filter(([, init]) => (init as RequestInit | undefined)?.method === 'PATCH').map(([, init]) => JSON.parse(String((init as RequestInit).body)))

afterEach(() => vi.unstubAllGlobals())

describe('ItemEditors', () => {
  it('saves a field as soon as it changes', async () => {
    const fetch = renderEditors()
    await userEvent.click(await within(await screen.findByRole('group', { name: 'Priority' })).findByRole('button', { name: 'high' }))
    await waitFor(() => expect(patches(fetch)[0]).toEqual({ path: 'tree/p/tasks/T001.md', changes: { priority: { from: null, to: 'high' } } }))
  })

  it('asks for whom before moving to a waiting status', async () => {
    const fetch = renderEditors()
    await screen.findByRole('option', { name: 'Waiting' })
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Status' }), 'waiting')
    expect(await screen.findByText('Waiting for whom?')).toBeInTheDocument()
    expect(patches(fetch)).toEqual([])
    await userEvent.click(screen.getByRole('checkbox', { name: /Keyvan Coen/ }))
    await userEvent.click(screen.getByRole('button', { name: 'Wait' }))
    await waitFor(() => expect(patches(fetch)[0].changes.waiting_for).toEqual({ from: null, to: ['keyvan'] }))
    expect(patches(fetch)[0].changes.status).toEqual({ from: 'todo', to: 'waiting' })
  })

  it('gives a ressource no status', async () => {
    renderEditors(makeItem({ kind: 'ressource', status: null, category: null }))
    await screen.findByText('Status').catch(() => null)
    expect(screen.queryByRole('combobox', { name: 'Status' })).not.toBeInTheDocument()
  })
})
