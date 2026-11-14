import { QueryClient, QueryClientProvider, useQuery } from '@tanstack/react-query'
import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { makeItem, queryKeys, type NodeSummary, type Schema } from '@/shared/api'
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
    estimates: { XS: 2, S: 4, M: 12, L: 24, XL: 48, default: 'M', test_ratio: 0.25, tester: 'owner' },
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

function renderEditors(item = makeItem(), saved: () => Promise<unknown> = async () => item) {
  const fetch = vi.fn(async (url: string, init?: RequestInit) => {
    const body = url.startsWith('/api/nodes') ? nodes : url.startsWith('/api/schema') ? schema : init?.method === 'PATCH' ? await saved() : []
    return new Response(JSON.stringify(body), { headers: { 'Content-Type': 'application/json' } })
  })
  vi.stubGlobal('fetch', fetch)
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  client.setQueryData(queryKeys.items, [item])
  function Editors() {
    const { data: items = [] } = useQuery({ queryKey: queryKeys.items, queryFn: () => [item] })
    return <ItemEditors item={items[0] ?? item} />
  }
  render(
    <QueryClientProvider client={client}>
      <Editors />
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

  it('shows the hours of the size, and the sum of a split story with its guesses', async () => {
    renderEditors(
      makeItem({
        kind: 'story',
        size: 'XS',
        estimate: { hours: 36, remaining_hours: 24, test_hours: 9, remaining_test_hours: 6, tester: 'others', basis: 'tasks', size: null, parts: 3, guessed: 1 },
      }),
    )
    const estimate = await screen.findByLabelText('Estimate')
    expect(within(estimate).getByText('~36 h')).toHaveAttribute('data-guessed', 'true')
    expect(estimate).toHaveTextContent('Sum of 3 parts, 1 guessed from a default or provisional size')
    expect(estimate).toHaveTextContent('+9 h of testing by others, as a delay')
    expect(estimate).toHaveTextContent('24 h remaining')
  })

  it('shows the hours of a new size before the server answers', async () => {
    const estimate = { hours: 12, remaining_hours: 12, test_hours: 3, remaining_test_hours: 3, tester: 'owner' as const, basis: 'size' as const, size: 'M', parts: 1, guessed: 0 }
    renderEditors(makeItem({ size: 'M', estimate }), () => new Promise(() => {}))
    expect(await screen.findByLabelText('Estimate')).toHaveTextContent('12 h')
    await screen.findByText('Status')
    await userEvent.click(within(screen.getByRole('group', { name: 'Size' })).getByRole('button', { name: 'XS' }))
    await waitFor(() => expect(screen.getByLabelText('Estimate')).toHaveTextContent('Size XS'))
    const resized = screen.getByLabelText('Estimate')
    expect(within(resized).getByText('2 h')).toBeInTheDocument()
    expect(resized).toHaveTextContent('+0.5 h of testing by the owner')
  })
})
