import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { cleanup, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { makeItem, type Directory, type PersonaDirectory } from '@/shared/api'
import { PeoplePage } from './people-page'

const directories: Directory[] = [
  {
    node: 'cmc',
    tracker: 'monday',
    account: 'acme',
    people: [{ handle: 'keyvan', name: 'Keyvan Coen', monday: 42, awaited_by: ['cmc/lib#T1'] }],
  },
]

const personas: PersonaDirectory[] = [{ node: 'cmc', personas: [{ id: 'user:rh', name: 'Responsable RH', used_by: ['cmc/lib#S1'] }] }]

function renderPage(url = '/people') {
  const fetch = vi.fn(async (url: string, init?: RequestInit) => {
    const body = url.startsWith('/api/people')
      ? directories
      : url.startsWith('/api/personas')
        ? personas
      : url.startsWith('/api/monday/users')
        ? [{ id: 7, name: 'Doryan B.', email: null }]
        : url.startsWith('/api/schema')
          ? { version: 1, categories: [], id_prefixes: {}, keys: [], reflexion_statuses: [] }
          : url.startsWith('/api/items')
          ? [
              makeItem({ node: 'cmc/lib', id: 'T1', title: 'Relire la PR', waiting_for: ['keyvan'] }),
              makeItem({ node: 'cmc/lib', id: 'S1', kind: 'story', title: 'Exporter les résultats', personas: ['user:rh'] }),
            ]
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

  it('lists the personas on their tab, with the stories written for them', async () => {
    renderPage('/people?tab=personas')
    expect(await screen.findByText('Responsable RH')).toBeInTheDocument()
    expect(screen.getByText('cmc/lib#S1')).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: 'Personas' })).toHaveAttribute('aria-selected', 'true')
    expect(screen.queryByText('Keyvan Coen')).not.toBeInTheDocument()
  })

  it('adds a persona with its group and role', async () => {
    const fetch = renderPage()
    await userEvent.click(await screen.findByRole('tab', { name: 'Personas' }))
    await userEvent.click(await screen.findByRole('button', { name: /Add a persona/ }))
    await userEvent.type(screen.getByRole('textbox', { name: 'Name' }), 'Développeur CMC')
    await userEvent.type(screen.getByRole('textbox', { name: /^Id/ }), 'cmc:dev')
    await userEvent.click(screen.getByRole('button', { name: 'Add' }))
    await waitFor(() => {
      const post = fetch.mock.calls.find(([, init]) => (init as RequestInit | undefined)?.method === 'POST')
      expect(post?.[0]).toBe('/api/personas')
      expect(JSON.parse(String((post?.[1] as RequestInit).body))).toEqual({ node: 'cmc', id: 'cmc:dev', name: 'Développeur CMC' })
    })
  })
})
