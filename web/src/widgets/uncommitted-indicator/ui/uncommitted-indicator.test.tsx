import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen } from '@testing-library/react'
import type { Repo } from '@/shared/api'
import { UncommittedIndicator } from './uncommitted-indicator'

function renderWith(repo: Repo) {
  vi.stubGlobal(
    'fetch',
    vi.fn(async () => new Response(JSON.stringify(repo), { headers: { 'Content-Type': 'application/json' } })),
  )
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={client}>
      <UncommittedIndicator />
    </QueryClientProvider>,
  )
}

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('UncommittedIndicator', () => {
  it('stays hidden when the working tree is clean', async () => {
    const { container } = renderWith({ root: '/r', branch: 'main', uncommitted: [] })
    await new Promise((resolve) => setTimeout(resolve, 20))
    expect(container).toBeEmptyDOMElement()
  })

  it('shows the number of uncommitted files', async () => {
    renderWith({
      root: '/r',
      branch: 'main',
      uncommitted: [
        { code: 'M', path: 'tree/a.md' },
        { code: '??', path: 'tree/b.md' },
      ],
    })
    expect(await screen.findByRole('button', { name: '2 uncommitted files' })).toHaveTextContent('2')
  })
})
