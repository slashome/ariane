import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
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
    const { container } = renderWith({ root: '/r', branch: 'main', ahead: 0, uncommitted: [] })
    await new Promise((resolve) => setTimeout(resolve, 20))
    expect(container).toBeEmptyDOMElement()
  })

  it('shows the number of uncommitted files', async () => {
    renderWith({
      root: '/r',
      branch: 'main',
      ahead: 0,
      uncommitted: [
        { code: 'M', path: 'tree/a.md' },
        { code: '??', path: 'tree/b.md' },
      ],
    })
    expect(await screen.findByRole('button', { name: '2 uncommitted files' })).toHaveTextContent('2')
  })

  it('stays visible after a commit while commits remain to push', async () => {
    renderWith({ root: '/r', branch: 'main', ahead: 2, uncommitted: [] })
    expect(await screen.findByRole('button', { name: '2 commits to push' })).toHaveTextContent('2')
  })

  it('asks for confirmation before discarding one file, then posts its path', async () => {
    const repo: Repo = {
      root: '/r',
      branch: 'main',
      ahead: 0,
      uncommitted: [
        { code: ' M', path: 'tree/a.md' },
        { code: '??', path: 'notes.md' },
      ],
    }
    const fetch = vi.fn(async (url: string) => {
      const body = url === '/api/discard' ? { discarded: ['tree/a.md'] } : repo
      return new Response(JSON.stringify(body), { headers: { 'Content-Type': 'application/json' } })
    })
    vi.stubGlobal('fetch', fetch)
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    render(
      <QueryClientProvider client={client}>
        <UncommittedIndicator />
      </QueryClientProvider>,
    )
    const user = userEvent.setup()
    await user.hover(await screen.findByRole('button', { name: '2 uncommitted files' }))
    expect(await screen.findByRole('button', { name: 'Discard all' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Discard notes.md' })).not.toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Discard tree/a.md' }))
    const dialog = await screen.findByRole('alertdialog')
    expect(fetch).not.toHaveBeenCalledWith('/api/discard', expect.anything())
    await user.click(within(dialog).getByRole('button', { name: 'Discard' }))
    expect(fetch).toHaveBeenCalledWith(
      '/api/discard',
      expect.objectContaining({ method: 'POST', body: JSON.stringify({ paths: ['tree/a.md'] }) }),
    )
  })
})
