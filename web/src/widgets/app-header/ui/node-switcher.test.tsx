import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { useItemFilters } from '@/features/filter-items'
import type { NodeSummary } from '@/shared/api'
import { NodeSwitcher } from './node-switcher'

const nodes: NodeSummary[] = ['', 'cmc', 'cmc/cmc-lib', 'cmc/cmc-frontend', 'slashome', 'slashome/ariane'].map((path, index) => ({
  path,
  has_index: true,
  items: index,
  workflow: [],
}))

function Scope() {
  const { filters } = useItemFilters()
  return <output aria-label="scope">{`${filters.node ?? '*'} ${filters.includeDescendants ? 'with' : 'without'} sub-nodes`}</output>
}

function renderAt(url: string) {
  vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify(nodes), { headers: { 'Content-Type': 'application/json' } })))
  const router = createMemoryRouter([{ path: '*', element: (<><NodeSwitcher /><Scope /></>) }], { initialEntries: [url] })
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

describe('NodeSwitcher', () => {
  it('lists the nodes as a tree and shows the full path of the one in scope', async () => {
    renderAt('/?node=cmc/cmc-lib')
    const select = screen.getByRole('combobox', { name: 'Node' })
    await within(select).findByRole('option', { name: /cmc-lib/ })
    const labels = within(select).getAllByRole('option').map((option) => option.textContent?.replace(/\u00a0/g, ' '))
    expect(labels).toEqual([
      'All nodes',
      '(home) (0)',
      'cmc (1)',
      '├─ cmc-frontend (3)',
      '└─ cmc-lib (2)',
      'slashome (4)',
      '└─ ariane (5)',
    ])
    expect(screen.getByText('cmc/cmc-lib')).toBeInTheDocument()
    expect(screen.queryByRole('switch', { name: 'Include the sub-nodes' })).not.toBeInTheDocument()
  })

  it('offers the sub-nodes toggle on a parent, off by default and remembered', async () => {
    renderAt('/?node=cmc')
    const toggle = await screen.findByRole('switch', { name: 'Include the sub-nodes' })
    expect(toggle).toHaveAttribute('aria-checked', 'false')
    expect(screen.getByLabelText('scope')).toHaveTextContent('cmc without sub-nodes')
    await userEvent.click(toggle)
    expect(screen.getByLabelText('scope')).toHaveTextContent('cmc with sub-nodes')
    expect(localStorage.getItem('ariane.scope.descendants')).toBe('1')
    await userEvent.selectOptions(screen.getByRole('combobox', { name: 'Node' }), '*')
    expect(screen.queryByRole('switch', { name: 'Include the sub-nodes' })).not.toBeInTheDocument()
  })
})
