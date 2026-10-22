import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { useThemePreview } from '@/entities/theme'
import type { Settings } from '@/shared/api'
import { SettingsPage } from './settings-page'

const saved: Settings = { version: 1, theme: { primary: '#3760c9', secondary: '#3760c9' }, dashboard: { pinned: [] } }

function Previewed() {
  const preview = useThemePreview()
  return <output aria-label="preview">{preview ? `${preview.primary} ${preview.secondary}` : 'none'}</output>
}

function renderPage() {
  const fetch = vi.fn(async (url: string, init?: RequestInit) => {
    const body = url === '/api/settings/theme' ? { ...saved, theme: JSON.parse(String(init?.body)) } : saved
    return new Response(JSON.stringify(body), { headers: { 'Content-Type': 'application/json' } })
  })
  vi.stubGlobal('fetch', fetch)
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const view = render(
    <QueryClientProvider client={client}>
      <SettingsPage />
      <Previewed />
    </QueryClientProvider>,
  )
  return { fetch, view }
}

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('SettingsPage', () => {
  it('previews a preset everywhere, then saves it to settings.toml', async () => {
    const { fetch } = renderPage()
    await userEvent.click(await screen.findByRole('button', { name: /Teal & coral/ }))
    expect(screen.getByLabelText('preview')).toHaveTextContent('#0f766e #e8604c')
    await userEvent.click(screen.getByRole('button', { name: 'Save' }))
    await waitFor(() =>
      expect(fetch).toHaveBeenCalledWith('/api/settings/theme', expect.objectContaining({ method: 'PUT', body: JSON.stringify({ primary: '#0f766e', secondary: '#e8604c' }) })),
    )
  })

  it('takes a typed color only once complete, and drops the preview when left', async () => {
    const { view } = renderPage()
    const field = await screen.findByLabelText('Primary')
    await userEvent.clear(field)
    await userEvent.type(field, '#aa00')
    expect(screen.getByLabelText('preview')).toHaveTextContent('none')
    await userEvent.type(field, '00')
    expect(screen.getByLabelText('preview')).toHaveTextContent('#aa0000 #3760c9')
    view.unmount()
    render(<Previewed />)
    expect(screen.getByLabelText('preview')).toHaveTextContent('none')
  })
})
