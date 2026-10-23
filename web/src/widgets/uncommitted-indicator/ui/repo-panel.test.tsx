import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { Repo } from '@/shared/api'
import { RepoPanel } from './repo-panel'

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('RepoPanel', () => {
  it('pulls the content when asked, even with nothing to push', async () => {
    const repo: Repo = { root: '/r', branch: 'main', ahead: 0, uncommitted: [] }
    const fetch = vi.fn(async (url: string) => {
      const body = url === '/api/pull' ? { pulled: 2 } : repo
      return new Response(JSON.stringify(body), { headers: { 'Content-Type': 'application/json' } })
    })
    vi.stubGlobal('fetch', fetch)
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    render(
      <QueryClientProvider client={client}>
        <RepoPanel />
      </QueryClientProvider>,
    )
    const pull = await screen.findByRole('button', { name: 'Pull' })
    await userEvent.click(pull)
    expect(fetch).toHaveBeenCalledWith('/api/pull', expect.objectContaining({ method: 'POST' }))
  })
})
