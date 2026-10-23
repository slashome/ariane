import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider, useSearchParams } from 'react-router'
import { nodes, schema } from '../model/vocabulary.fixture'
import { QueryField } from './query-field'

function Query() {
  const [params] = useSearchParams()
  return <output aria-label="query">{params.get('q')}</output>
}

function renderField(url = '/') {
  vi.stubGlobal(
    'fetch',
    vi.fn(async (input: string) => {
      const body = input.endsWith('/api/schema') ? schema : input.endsWith('/api/nodes') ? nodes : []
      return new Response(JSON.stringify(body), { headers: { 'Content-Type': 'application/json' } })
    }),
  )
  const router = createMemoryRouter(
    [
      {
        path: '/',
        element: (
          <>
            <QueryField count={2} total={5} />
            <Query />
          </>
        ),
      },
    ],
    { initialEntries: [url] },
  )
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  )
  return screen.getByRole('combobox', { name: 'Search items' })
}

afterEach(() => vi.unstubAllGlobals())

describe('QueryField', () => {
  it('suggests the keys, then the values, chosen with the arrows and Enter, and writes ?q=', async () => {
    const field = renderField()
    await userEvent.click(field)
    await userEvent.type(field, 'st')
    expect(await screen.findByRole('option', { name: /status:/ })).toHaveAttribute('aria-selected', 'true')
    await userEvent.keyboard('{Enter}')
    expect(field).toHaveValue('status:')
    expect(await screen.findByRole('option', { name: /fait/ })).toBeInTheDocument()
    await userEvent.keyboard('{ArrowDown}{Enter}')
    expect(field).toHaveValue('status:fait ')
    expect(screen.getByLabelText('query')).toHaveTextContent('status:fait')
  })

  it('reports what it does not know, and counts the items against the view', async () => {
    const field = renderField('/?q=status:zzz%20foo:bar')
    expect(await screen.findByLabelText('2 problems in the search')).toBeInTheDocument()
    expect(field).toHaveAttribute('aria-invalid', 'true')
    expect(field).toHaveAccessibleDescription(/no status “zzz”.*“foo:” is not a filter/)
    expect(screen.getByText('2 of 5')).toBeInTheDocument()
  })

  it('closes the suggestions on Escape', async () => {
    const field = renderField()
    await userEvent.click(field)
    expect(await screen.findByRole('listbox')).toBeInTheDocument()
    await userEvent.keyboard('{Escape}')
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument()
    expect(field).toHaveAttribute('aria-expanded', 'false')
  })
})
