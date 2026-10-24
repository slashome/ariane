import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { ProxyReport } from '@/shared/api'
import { ProxyPage } from './proxy-page'

const report: ProxyReport = {
  ok: false,
  proxy_port: 27380,
  server_port: 27317,
  checks: [
    { ok: true, what: 'alias 127.0.0.2 on lo0', hint: 'missing: run `ariane proxy setup`' },
    { ok: true, what: 'front door on 127.0.0.1:27380 or [::1]:27380', hint: '' },
    { ok: true, what: '127.0.0.2:80 redirected to the front door', hint: '' },
    { ok: true, what: 'ariane.local → 127.0.0.2 in /etc/hosts', hint: 'run `ariane proxy setup` to declare it' },
    { ok: true, what: 'ariane.local → 127.0.0.1:27317, the interface (`server.host`)', hint: 'nothing listens: start its server' },
    { ok: true, what: 'dreamcatchr.local → 127.0.0.2 in /etc/hosts', hint: 'run `ariane proxy setup` to declare it' },
    { ok: false, what: 'dreamcatchr.local → 127.0.0.1:3000, node slashome/dreamcatchr', hint: 'nothing listens: start its server' },
  ],
  routes: [
    { host: 'ariane.local', port: 27317, node: null, declared: true, listening: true },
    { host: 'dreamcatchr.local', port: 3000, node: 'slashome/dreamcatchr', declared: true, listening: false },
  ],
  problems: [],
}

const healthy: ProxyReport = {
  ...report,
  ok: true,
  checks: report.checks.map((check) => ({ ...check, ok: true })),
  routes: report.routes.map((route) => ({ ...route, listening: true })),
}

function renderPage(served: ProxyReport = report) {
  const fetch = vi.fn(async (url: string, init?: RequestInit) => {
    void init
    const body = url.startsWith('/api/proxy/routes')
      ? { routes: [], problems: [] }
      : url.startsWith('/api/proxy')
        ? served
        : url.startsWith('/api/nodes')
          ? [{ path: 'slashome/dreamcatchr' }, { path: 'cmc/lib' }]
          : []
    return new Response(JSON.stringify(body), { headers: { 'Content-Type': 'application/json' } })
  })
  vi.stubGlobal('fetch', fetch)
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <ProxyPage />
    </QueryClientProvider>,
  )
  return fetch
}

const sent = (fetch: ReturnType<typeof renderPage>, method: string) =>
  fetch.mock.calls.filter(([url, init]) => String(url).startsWith('/api/proxy/routes') && (init as RequestInit | undefined)?.method === method)

const bodyOf = (calls: ReturnType<typeof sent>) => JSON.parse(String((calls[0]?.[1] as RequestInit | undefined)?.body)) as unknown

afterEach(() => vi.unstubAllGlobals())

describe('ProxyPage', () => {
  it('shows each link of the chain, the hint only where it is broken, and the verdict', async () => {
    renderPage()
    expect(await screen.findByText('1 link broken')).toBeInTheDocument()
    expect(screen.getByText('alias 127.0.0.2 on lo0')).toBeInTheDocument()
    expect(screen.getAllByText('nothing listens: start its server')).toHaveLength(1)
    expect(screen.queryByText('missing: run `ariane proxy setup`')).not.toBeInTheDocument()
  })

  it('lists the routes with their owner and a link to each name, the interface read-only', async () => {
    renderPage()
    const link = await screen.findByRole('link', { name: 'Open http://dreamcatchr.local in a new tab' })
    expect(link).toHaveAttribute('href', 'http://dreamcatchr.local')
    expect(link).toHaveAttribute('target', '_blank')
    const server = screen.getByRole('row', { name: /^Open http:\/\/ariane\.local / })
    expect(within(server).getByText('interface')).toBeInTheDocument()
    expect(within(server).getByLabelText('Read-only: from config.toml')).toBeInTheDocument()
    expect(within(server).queryByRole('button', { name: /Edit/ })).not.toBeInTheDocument()
    const node = screen.getByRole('row', { name: /^Open http:\/\/dreamcatchr\.local / })
    expect(within(node).getByText('slashome/dreamcatchr')).toBeInTheDocument()
    expect(within(node).getByText('not listening')).toBeInTheDocument()
    expect(within(node).getByRole('button', { name: 'Edit dreamcatchr.local' })).toBeInTheDocument()
  })

  it('offers the root commands to copy, never to run', async () => {
    renderPage()
    expect(await screen.findByRole('button', { name: 'Copy ariane proxy setup' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Copy ariane proxy teardown' })).toBeInTheDocument()
    expect(screen.getByText(/The interface never runs them/)).toBeInTheDocument()
  })

  it('checks again on Re-check', async () => {
    const fetch = renderPage()
    await screen.findByText('1 link broken')
    const before = fetch.mock.calls.filter(([url]) => url === '/api/proxy').length
    await userEvent.click(screen.getByRole('button', { name: 'Re-check' }))
    await waitFor(() => expect(fetch.mock.calls.filter(([url]) => url === '/api/proxy').length).toBe(before + 1))
  })

  it('adds a route with a suggested host, refusing a taken port, and reminds to run setup', async () => {
    const fetch = renderPage()
    await userEvent.click(await screen.findByRole('button', { name: /Add a route/ }))
    await screen.findByRole('option', { name: 'cmc/lib' })
    await userEvent.selectOptions(screen.getByRole('combobox'), 'cmc/lib')
    expect(screen.getByRole('textbox', { name: /^Host/ })).toHaveValue('lib.local')
    await userEvent.type(screen.getByRole('textbox', { name: /^Port/ }), '27380')
    expect(screen.getByRole('alert')).toHaveTextContent("27380 is the front door's port (proxy.port).")
    expect(screen.getByRole('button', { name: 'Add' })).toBeDisabled()
    await userEvent.clear(screen.getByRole('textbox', { name: /^Port/ }))
    await userEvent.type(screen.getByRole('textbox', { name: /^Port/ }), '6006')
    await userEvent.click(screen.getByRole('button', { name: 'Add' }))
    await waitFor(() => expect(sent(fetch, 'PUT')).toHaveLength(1))
    expect(bodyOf(sent(fetch, 'PUT'))).toEqual({ node: 'cmc/lib', host: 'lib.local', port: 6006 })
    expect(await screen.findByText(/is not in \/etc\/hosts yet/)).toBeInTheDocument()
  })

  it('removes a node route after confirmation', async () => {
    const fetch = renderPage()
    await userEvent.click(await screen.findByRole('button', { name: 'Remove dreamcatchr.local' }))
    await userEvent.click(screen.getByRole('button', { name: 'Remove' }))
    await waitFor(() => expect(sent(fetch, 'DELETE')).toHaveLength(1))
    expect(bodyOf(sent(fetch, 'DELETE'))).toEqual({ node: 'slashome/dreamcatchr' })
  })
  it('folds the chain and the root step once every link holds, and unfolds them on demand', async () => {
    renderPage(healthy)
    const chain = await screen.findByRole('button', { name: 'The chain' })
    expect(chain).toHaveAttribute('aria-expanded', 'false')
    expect(screen.getByText('Every link holds · 7 checks')).toBeInTheDocument()
    expect(screen.queryByText('alias 127.0.0.2 on lo0')).not.toBeInTheDocument()
    const root = screen.getByRole('button', { name: 'The root step' })
    expect(root).toHaveAttribute('aria-expanded', 'false')
    expect(screen.queryByRole('button', { name: 'Copy ariane proxy setup' })).not.toBeInTheDocument()
    await userEvent.click(chain)
    expect(chain).toHaveAttribute('aria-expanded', 'true')
    expect(screen.getByText('alias 127.0.0.2 on lo0')).toBeInTheDocument()
    await userEvent.click(root)
    expect(screen.getByRole('button', { name: 'Copy ariane proxy setup' })).toBeInTheDocument()
  })

  it('opens the root step when a name is missing from /etc/hosts, the chain staying folded', async () => {
    renderPage({ ...healthy, routes: healthy.routes.map((route) => (route.node === null ? route : { ...route, declared: false })) })
    expect(await screen.findByRole('button', { name: 'The root step' })).toHaveAttribute('aria-expanded', 'true')
    expect(screen.getByRole('button', { name: 'The chain' })).toHaveAttribute('aria-expanded', 'false')
  })

  it('lists the routes before the chain', async () => {
    renderPage()
    const routes = await screen.findByRole('heading', { name: /^Routes/ })
    const chain = screen.getByRole('button', { name: 'The chain' })
    expect(routes.compareDocumentPosition(chain) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })
})
