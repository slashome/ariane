import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider, useLocation } from 'react-router'
import { makeItem, type SearchHit } from '@/shared/api'
import { CommandPalette } from './command-palette'

const items = [
  makeItem({ id: 'T001', node: 'p', title: 'Préparer la rentrée', external: ['monday:42'] }),
  makeItem({ id: 'S001', node: 'p', kind: 'story', title: 'Une histoire', path: 'tree/p/stories/S001-a.md' }),
]

const hit: SearchHit = { ...makeItem({ id: 'T009', node: 'q', title: 'Dans le corps', path: 'tree/q/tasks/T009.md' }), field: 'body', excerpt: 'il faut ranger la cave', highlights: [[8, 14]] }

function Where() {
  const { pathname, search } = useLocation()
  return <output aria-label="location">{`${pathname}${search}`}</output>
}

function renderPalette() {
  const fetch = vi.fn(async (input: string) => {
    const body = input.startsWith('/api/search') ? [hit] : input.startsWith('/api/items') ? items : input.startsWith('/api/nodes') ? [{ path: 'p', items: 2, workflow: [] }] : []
    return new Response(JSON.stringify(body), { headers: { 'Content-Type': 'application/json' } })
  })
  vi.stubGlobal('fetch', fetch)
  const router = createMemoryRouter(
    [
      {
        path: '*',
        element: (
          <>
            <CommandPalette />
            <Where />
          </>
        ),
      },
    ],
    { initialEntries: ['/tasks/list?node=p'] },
  )
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  )
  return fetch
}

const location = () => screen.getByLabelText('location').textContent

async function openPalette() {
  await userEvent.keyboard('{Control>}k{/Control}')
  return screen.findByRole('combobox', { name: /Search items, views/ })
}

afterEach(() => vi.unstubAllGlobals())

describe('CommandPalette', () => {
  it('opens with Ctrl+K, lists the views and actions, and opens a view with Enter, the scope kept', async () => {
    renderPalette()
    const input = await openPalette()
    expect(screen.getByRole('group', { name: 'Views' })).toBeInTheDocument()
    expect(screen.queryByRole('group', { name: 'Items' })).not.toBeInTheDocument()
    await userEvent.type(input, 'kanb')
    expect(screen.getByRole('option', { name: /Kanban/ })).toHaveAttribute('aria-selected', 'true')
    expect(input).toHaveAttribute('aria-activedescendant', screen.getByRole('option', { name: /Kanban/ }).id)
    await userEvent.keyboard('{Enter}')
    expect(screen.queryByRole('combobox')).not.toBeInTheDocument()
    expect(location()).toBe('/tasks/kanban?node=p')
  })

  it('opens an item by id: its page with Enter, its preview with Shift+Enter', async () => {
    renderPalette()
    let input = await openPalette()
    await userEvent.type(input, 'p#T001')
    expect(await screen.findByRole('option', { name: /Préparer la rentrée/ })).toHaveAttribute('aria-selected', 'true')
    await userEvent.keyboard('{Shift>}{Enter}{/Shift}')
    expect(location()).toBe('/tasks/list?node=p&preview=tree%2Fp%2Ftasks%2FT001.md')
    input = await openPalette()
    await userEvent.type(input, 'histoire')
    await screen.findByRole('option', { name: /Une histoire/ })
    await userEvent.keyboard('{Enter}')
    expect(location()).toBe('/stories/p/S001?node=p')
  })

  it('finds the text of the bodies through ariane search, with its excerpt highlighted', async () => {
    const fetch = renderPalette()
    const input = await openPalette()
    await userEvent.type(input, 'ranger')
    const group = await screen.findByRole('group', { name: 'In the text' })
    expect(within(group).getByText('ranger').tagName).toBe('MARK')
    expect(fetch).toHaveBeenCalledWith('/api/search?q=ranger', expect.anything())
  })

  it('opens the list of items bound to monday, and goes back with Escape', async () => {
    renderPalette()
    const input = await openPalette()
    await userEvent.type(input, 'monday')
    await userEvent.click(await screen.findByRole('option', { name: /Send an item’s status to monday/ }))
    expect(screen.getByRole('combobox', { name: /Send an item’s status to monday/ })).toBeInTheDocument()
    expect(screen.getByRole('option', { name: /Préparer la rentrée/ })).toBeInTheDocument()
    await userEvent.keyboard('{Escape}')
    expect(screen.getByRole('combobox', { name: /Search items, views/ })).toBeInTheDocument()
    expect(screen.queryByRole('option', { name: /Préparer la rentrée/ })).not.toBeInTheDocument()
    await userEvent.keyboard('{Escape}')
    expect(screen.queryByRole('combobox')).not.toBeInTheDocument()
  })

  it('moves with the arrows and wraps around', async () => {
    renderPalette()
    await openPalette()
    const options = screen.getAllByRole('option')
    expect(options[0]).toHaveAttribute('aria-selected', 'true')
    await userEvent.keyboard('{ArrowUp}')
    expect(screen.getAllByRole('option').at(-1)).toHaveAttribute('aria-selected', 'true')
    await userEvent.keyboard('{ArrowDown}{ArrowDown}')
    expect(screen.getAllByRole('option')[1]).toHaveAttribute('aria-selected', 'true')
  })
})
