import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { cleanup, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { makeItem, type Directory } from '@/shared/api'
import { PeoplePage } from './people-page'

const directories: Directory[] = [
  {
    node: 'cmc',
    tracker: 'monday',
    account: 'acme',
    people: [{ handle: 'keyvan', name: 'Keyvan Coen', monday: 42, awaited_by: ['cmc/lib#T1'] }],
  },
]

function renderPage(url = '/people') {
  const fetch = vi.fn(async (url: string, init?: RequestInit) => {
    const body = url.startsWith('/api/people')
      ? directories
      : url.startsWith('/api/monday/users')
        ? [{ id: 7, name: 'Doryan B.', email: null }]
        : url.startsWith('/api/schema')
          ? { version: 1, categories: [], id_prefixes: {}, keys: [], reflexion_statuses: [] }
          : url.startsWith('/api/items')
          ? [makeItem({ node: 'cmc/lib', id: 'T1', title: 'Relire la PR', waiting_for: ['keyvan'] })]
          : []
    void init
    return new Response(JSON.stringify(body), { headers: { 'Content-Type': 'application/json' } })
  })
  vi.stubGlobal('fetch', fetch)
  const router = createMemoryRouter([{ path: '/people', element: <PeoplePage /> }], { initialEntries: [url] })
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  )
  return fetch
}

afterEach(() => vi.unstubAllGlobals())

describe('PeoplePage', () => {
  it('lists the people of each tracker node and who waits for them', async () => {
    renderPage()
    expect(await screen.findByText('Keyvan Coen')).toBeInTheDocument()
    expect(screen.getByText('cmc/lib#T1')).toBeInTheDocument()
    expect(screen.getByText('monday · acme')).toBeInTheDocument()
  })

  it('keeps the people awaited by the items the search finds, highlighted', async () => {
    renderPage('/people?q=relire')
    expect(await screen.findByText('cmc/lib#T1')).toBeInTheDocument()
    expect(screen.getByText('1 of 1')).toBeInTheDocument()
    cleanup()
    renderPage('/people?q=absent')
    expect(await screen.findByText('No one here waited on by an item the search finds.')).toBeInTheDocument()
    expect(screen.queryByText('Keyvan Coen')).not.toBeInTheDocument()
  })

  it('adds a person found in monday, the handle suggested from the name', async () => {
    const fetch = renderPage()
    await userEvent.click(await screen.findByRole('button', { name: /Add a person/ }))
    await userEvent.type(screen.getByRole('textbox', { name: 'Search monday users by name' }), 'dor')
    await userEvent.click(screen.getByRole('button', { name: 'Search monday' }))
    await userEvent.click(await screen.findByRole('button', { name: /Doryan B\./ }))
    expect(screen.getByRole('textbox', { name: /^Handle/ })).toHaveValue('doryan')
    await userEvent.click(screen.getByRole('button', { name: 'Add' }))
    await waitFor(() => {
      const post = fetch.mock.calls.find(([, init]) => (init as RequestInit | undefined)?.method === 'POST')
      expect(JSON.parse(String((post?.[1] as RequestInit).body))).toEqual({ node: 'cmc', handle: 'doryan', name: 'Doryan B.', monday: 7 })
    })
  })
})
