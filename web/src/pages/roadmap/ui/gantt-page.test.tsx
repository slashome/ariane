import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { makeItem } from '@/shared/api'
import { sampleRoadmap } from '../model/fixtures'
import { GanttPage } from './gantt-page'

const items = [makeItem({ id: 'T2', node: 'p', title: 'Work p#T2', path: 'tree/p/tasks/T2.md' })]

function renderAt(url: string) {
  const fetch = vi.fn(async (input: string) => {
    const body = input.startsWith('/api/roadmap') ? sampleRoadmap() : input.startsWith('/api/items') ? items : {}
    return new Response(JSON.stringify(body), { headers: { 'Content-Type': 'application/json' } })
  })
  vi.stubGlobal('fetch', fetch)
  const router = createMemoryRouter([{ path: '/roadmap/gantt', element: <GanttPage /> }], { initialEntries: [url] })
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  )
  return { router, fetch }
}

afterEach(() => {
  vi.unstubAllGlobals()
})

function levelButton(name: string) {
  return within(screen.getByRole('group', { name: 'Levels' })).getByRole('button', { name })
}

describe('the Gantt page', () => {
  it('asks the forecast of the chosen node and draws a row per release, epic, story and task', async () => {
    const { fetch } = renderAt('/roadmap/gantt?node=p')
    const rows = await screen.findByRole('list', { name: 'Roadmap rows' })
    expect(fetch.mock.calls.some(([url]) => url === '/api/roadmap?node=p')).toBe(true)
    expect(within(rows).getAllByRole('listitem')).toHaveLength(14)
    expect(within(rows).getAllByText('0.1')).toHaveLength(1)
  })

  it('keeps at least one level shown and remembers the choice', async () => {
    renderAt('/roadmap/gantt?node=p')
    await screen.findByRole('list', { name: 'Roadmap rows' })
    for (const name of ['Releases', 'Stories', 'Tasks']) {
      await userEvent.click(levelButton(name))
    }
    expect(within(screen.getByRole('list', { name: 'Roadmap rows' })).getAllByRole('listitem')).toHaveLength(3)
    await userEvent.click(levelButton('Epics'))
    expect(levelButton('Epics')).toHaveAttribute('aria-pressed', 'true')
    expect(JSON.parse(localStorage.getItem('ariane.roadmap.levels') ?? '[]')).toEqual(['epic'])
  })

  it('draws an epic over two releases as two fragments on one row', async () => {
    localStorage.setItem('ariane.roadmap.levels', JSON.stringify(['epic']))
    renderAt('/roadmap/gantt?node=p')
    await screen.findByRole('list', { name: 'Roadmap rows' })
    expect(screen.getByRole('button', { name: 'E1 span in 0.1' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'E1 span in 0.2' })).toBeInTheDocument()
  })

  it('hatches guessed sizes, marks a missed target and opens the item of a bar', async () => {
    localStorage.setItem('ariane.roadmap.levels', JSON.stringify(['task']))
    const { router } = renderAt('/roadmap/gantt?node=p')
    await screen.findByRole('list', { name: 'Roadmap rows' })
    expect(screen.getByRole('button', { name: 'E2 bar' }).querySelector('[data-hatched]')).not.toBeNull()
    expect(screen.getByRole('button', { name: 'T2 bar' }).querySelector('[data-hatched]')).toBeNull()
    expect(screen.getAllByTestId('missed-target')).toHaveLength(1)
    await userEvent.click(screen.getByRole('button', { name: 'T2 bar' }))
    expect(new URLSearchParams(router.state.location.search).get('preview')).toBe('tree/p/tasks/T2.md')
  })

  it('moves through time even with no node chosen', async () => {
    renderAt('/roadmap/gantt')
    expect(screen.getByText('Choose a node in the header to see its roadmap')).toBeInTheDocument()
    expect(screen.getByTestId('now-line')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Later' }))
    await userEvent.click(screen.getByRole('button', { name: 'Later' }))
    expect(screen.queryByTestId('now-line')).not.toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Today' }))
    expect(screen.getByTestId('now-line')).toBeInTheDocument()
  })
})
