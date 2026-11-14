import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { makeItem, type NodeSummary, type Roadmap } from '@/shared/api'
import { sampleRoadmap } from '../model/fixtures'
import { GanttPage } from './gantt-page'

const items = [
  makeItem({ id: 'T1', node: 'p', title: 'Work p#T1', path: 'tree/p/tasks/T1.md', rank: 1000, category: 'done', size: 'S' }),
  makeItem({ id: 'T2', node: 'p', title: 'Work p#T2', path: 'tree/p/tasks/T2.md', rank: 2000, size: 'M' }),
  makeItem({ id: 'T3', node: 'p', title: 'Work p#T3', path: 'tree/p/tasks/T3.md', size: 'XS' }),
  makeItem({ id: 'T4', node: 'p', title: 'Work p#T4', path: 'tree/p/tasks/T4.md', size: 'S' }),
  makeItem({ id: 'T8', node: 'p', title: 'Work p#T8', path: 'tree/p/tasks/T8.md' }),
  makeItem({ id: 'S1', node: 'p', kind: 'story', path: 'tree/p/stories/S1.md' }),
  makeItem({ id: 'S2', node: 'p', kind: 'story', path: 'tree/p/stories/S2.md', size: 'M' }),
]

const nodes: Partial<NodeSummary>[] = [
  { path: 'p', workflow: [], items: items.length, estimates: { XS: 2, S: 4, M: 12, L: 24, XL: 48, default: 'M', test_ratio: 0.25, tester: 'owner' } },
]

function answer(input: string, init: RequestInit | undefined, roadmap: Roadmap): unknown {
  const method = init?.method ?? 'GET'
  if (input === '/api/items/rank' && method === 'POST') {
    return { items: [] }
  }
  if (input === '/api/items' && method === 'PATCH') {
    const { path } = JSON.parse(String(init?.body)) as { path: string }
    return items.find((item) => item.path === path)
  }
  if (input.startsWith('/api/roadmap')) {
    return roadmap
  }
  if (input.startsWith('/api/nodes')) {
    return nodes
  }
  return input.startsWith('/api/items') ? items : {}
}

function sent(fetch: ReturnType<typeof vi.fn>, method: string, url: string): unknown[] {
  return fetch.mock.calls
    .filter(([input, init]) => input === url && (init as RequestInit | undefined)?.method === method)
    .map(([, init]) => JSON.parse(String((init as RequestInit).body)) as unknown)
}

function roadmapFetches(fetch: ReturnType<typeof vi.fn>): number {
  return fetch.mock.calls.filter(([input]) => String(input).startsWith('/api/roadmap')).length
}

function richRoadmap(): Roadmap {
  const roadmap = sampleRoadmap()
  return {
    ...roadmap,
    unscheduled: [
      { reference: 'p#T8', node: 'p', kind: 'task', title: 'Parked work', reason: 'parking', release: 'plus-tard' },
      { reference: 'p#T9', node: 'p', kind: 'task', title: 'Loose work', reason: 'no_release', release: null },
    ],
    warnings: [
      ...roadmap.warnings,
      { kind: 'provisional', count: 1, items: [{ reference: 'p#S2', detail: null }] },
      { kind: 'without_release', count: 1, items: [{ reference: 'p#T9', detail: null }] },
    ],
  }
}

function renderAt(url: string, roadmap: Roadmap = sampleRoadmap()) {
  const fetch = vi.fn(async (input: string, init?: RequestInit) =>
    new Response(JSON.stringify(answer(input, init, roadmap)), { headers: { 'Content-Type': 'application/json' } }),
  )
  vi.stubGlobal('fetch', fetch)
  const router = createMemoryRouter(
    [
      { path: '/roadmap/gantt', element: <GanttPage /> },
      { path: '/roadmap/settings', element: <p>Settings page</p> },
    ],
    { initialEntries: [url] },
  )
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

  it('says when no calendar covers the node and links to the roadmap settings', async () => {
    const { router } = renderAt('/roadmap/gantt?node=p')
    const banner = await screen.findByRole('note', { name: 'No calendar' })
    expect(within(banner).getByText('ariane calendar edit')).toBeInTheDocument()
    await userEvent.click(within(banner).getByRole('link', { name: 'Roadmap › Settings' }))
    expect(router.state.location.pathname).toBe('/roadmap/settings')
    expect(router.state.location.search).toBe('?node=p')
  })

  it('shows no calendar banner when a calendar gives the dates', async () => {
    renderAt('/roadmap/gantt?node=p', { ...sampleRoadmap(), carrier: 'p' })
    await screen.findByRole('list', { name: 'Roadmap rows' })
    expect(screen.queryByRole('note', { name: 'No calendar' })).not.toBeInTheDocument()
  })

  it('counts each warning kind and filters the view on a counter until it is clicked again', async () => {
    renderAt('/roadmap/gantt?node=p', richRoadmap())
    const banner = await screen.findByRole('group', { name: 'Warnings' })
    expect(within(banner).getAllByRole('button').map((button) => button.getAttribute('aria-label'))).toEqual([
      'Missed target, 1',
      'No release, 1',
      'Provisional size, 1',
    ])
    const provisional = within(banner).getByRole('button', { name: 'Provisional size, 1' })
    await userEvent.click(provisional)
    expect(provisional).toHaveAttribute('aria-pressed', 'true')
    expect(within(screen.getByRole('list', { name: 'Roadmap rows' })).getAllByRole('listitem')).toHaveLength(3)
    expect(screen.queryByRole('button', { name: 'plus-tard, 1' })).not.toBeInTheDocument()
    await userEvent.click(provisional)
    expect(within(screen.getByRole('list', { name: 'Roadmap rows' })).getAllByRole('listitem')).toHaveLength(14)
  })

  it('points a filter on unscheduled work to the columns', async () => {
    renderAt('/roadmap/gantt?node=p', richRoadmap())
    await userEvent.click(await screen.findByRole('button', { name: 'No release, 1' }))
    expect(screen.getByText('Nothing scheduled carries this warning: see the columns on the right.')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Unplanned, 1' })).toBeInTheDocument()
  })

  it('hides the warnings banner when there is no warning', async () => {
    renderAt('/roadmap/gantt?node=p', { ...sampleRoadmap(), warnings: [] })
    await screen.findByRole('list', { name: 'Roadmap rows' })
    expect(screen.queryByRole('group', { name: 'Warnings' })).not.toBeInTheDocument()
  })

  it('folds the plus-tard and Unplanned columns by default, unfolds one on click and remembers it', async () => {
    const { router } = renderAt('/roadmap/gantt?node=p', richRoadmap())
    const parked = await screen.findByRole('button', { name: 'plus-tard, 1' })
    expect(screen.getByRole('button', { name: 'Unplanned, 1' })).toHaveAttribute('aria-expanded', 'false')
    await userEvent.click(parked)
    const column = screen.getByRole('region', { name: 'plus-tard' })
    expect(within(column).getByText('Parked work')).toBeInTheDocument()
    expect(JSON.parse(localStorage.getItem('ariane.roadmap.unfolded') ?? '[]')).toEqual(['parking:plus-tard'])
    await userEvent.click(within(column).getByRole('button', { name: /Parked work/ }))
    expect(new URLSearchParams(router.state.location.search).get('preview')).toBe('tree/p/tasks/T8.md')
  })

  it('opens a column the viewer left unfolded, with the reason of each unplanned item', async () => {
    localStorage.setItem('ariane.roadmap.unfolded', JSON.stringify(['unplanned']))
    renderAt('/roadmap/gantt?node=p', richRoadmap())
    const column = await screen.findByRole('region', { name: 'Unplanned' })
    expect(within(column).getByText('no release')).toBeInTheDocument()
    await userEvent.click(within(column).getByRole('button', { name: 'Fold Unplanned' }))
    expect(screen.getByRole('button', { name: 'Unplanned, 1' })).toBeInTheDocument()
    expect(JSON.parse(localStorage.getItem('ariane.roadmap.unfolded') ?? '[]')).toEqual([])
  })

  it('hatches a summary with a guessed part and prefixes its hours with a tilde', async () => {
    localStorage.setItem('ariane.roadmap.levels', JSON.stringify(['epic']))
    renderAt('/roadmap/gantt?node=p')
    const span = await screen.findByRole('button', { name: 'E1 span in 0.1' })
    expect(span).toHaveAttribute('data-guessed', 'true')
    expect(screen.getByRole('button', { name: 'E1 span in 0.2' })).toHaveAttribute('data-guessed', 'true')
    expect(screen.getByText('~40 h')).toBeInTheDocument()
  })

  it('shows an arrow at the edge for rows whose bars are out of view, and scrolls to them on click', async () => {
    localStorage.setItem('ariane.roadmap.levels', JSON.stringify(['task']))
    renderAt('/roadmap/gantt?node=p')
    await screen.findByRole('list', { name: 'Roadmap rows' })
    expect(screen.queryAllByRole('button', { name: /^Scroll to/ })).toHaveLength(0)
    await userEvent.click(screen.getByRole('button', { name: 'Later' }))
    expect(screen.getByRole('button', { name: 'Scroll to T4, earlier' })).toHaveAttribute('data-edge', 'before')
    expect(screen.queryByRole('button', { name: 'T4 bar' })).not.toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: 'Scroll to T4, earlier' }))
    expect(screen.getByRole('button', { name: 'T4 bar' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Scroll to T4, earlier' })).not.toBeInTheDocument()
  })

  it('moves a row before its previous sibling with Alt+↑, through the rank route of the kanban, then refreshes the forecast', async () => {
    const { fetch } = renderAt('/roadmap/gantt?node=p')
    const grip = await screen.findByRole('button', { name: 'Reorder T2' })
    await waitFor(() => expect(screen.getByRole('button', { name: 'Resize T2' })).toBeInTheDocument())
    const before = roadmapFetches(fetch)
    grip.focus()
    await userEvent.keyboard('{Alt>}{ArrowUp}{/Alt}')
    await waitFor(() =>
      expect(sent(fetch, 'POST', '/api/items/rank')).toEqual([
        { path: 'tree/p/tasks/T2.md', place: 'before', other: 'tree/p/tasks/T1.md', seen: { 'tree/p/tasks/T2.md': 2000, 'tree/p/tasks/T1.md': 1000 } },
      ]),
    )
    expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument()
    await waitFor(() => expect(roadmapFetches(fetch)).toBeGreaterThan(before))
  })

  it('takes the reorder grip away from done rows', async () => {
    renderAt('/roadmap/gantt?node=p')
    await screen.findByRole('button', { name: 'Reorder T2' })
    expect(screen.queryByRole('button', { name: 'Reorder T1' })).not.toBeInTheDocument()
  })

  it('asks before a move that ranks every unranked item of the node, and sends it only once confirmed', async () => {
    const { fetch } = renderAt('/roadmap/gantt?node=p')
    const grip = await screen.findByRole('button', { name: 'Reorder S1' })
    await waitFor(() => expect(screen.getByRole('button', { name: 'Resize T2' })).toBeInTheDocument())
    grip.focus()
    await userEvent.keyboard('{Alt>}{ArrowDown}{/Alt}')
    const dialog = await screen.findByRole('alertdialog')
    expect(dialog).toHaveTextContent('This move ranks the 2 items of this node: 2 files modified, not committed. Continue?')
    await userEvent.click(within(dialog).getByRole('button', { name: 'Cancel' }))
    expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument()
    expect(sent(fetch, 'POST', '/api/items/rank')).toEqual([])
    grip.focus()
    await userEvent.keyboard('{Alt>}{ArrowDown}{/Alt}')
    await userEvent.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'Continue' }))
    await waitFor(() =>
      expect(sent(fetch, 'POST', '/api/items/rank')).toEqual([
        { path: 'tree/p/stories/S1.md', place: 'after', other: 'tree/p/stories/S2.md', seen: { 'tree/p/stories/S1.md': null, 'tree/p/stories/S2.md': null } },
      ]),
    )
  })

  it('reorders a row dropped on a sibling and refuses a drop in another group', async () => {
    const { fetch } = renderAt('/roadmap/gantt?node=p')
    const grip = await screen.findByRole('button', { name: 'Reorder T2' })
    await waitFor(() => expect(screen.getByRole('button', { name: 'Resize T2' })).toBeInTheDocument())
    const rowOf = (name: string) => screen.getByRole('button', { name }).closest('li') as HTMLElement
    const pointAt = vi.fn(() => rowOf('Reorder T4'))
    document.elementFromPoint = pointAt
    fireEvent.pointerDown(grip, { pointerId: 1, clientY: 0 })
    fireEvent.pointerMove(grip, { pointerId: 1, clientY: 5 })
    expect(await screen.findByRole('status')).toHaveTextContent('Drop it among its siblings')
    pointAt.mockImplementation(() => rowOf('T1 bar'))
    fireEvent.pointerMove(grip, { pointerId: 1, clientY: -5 })
    expect(screen.queryByRole('status')).not.toBeInTheDocument()
    fireEvent.pointerUp(grip, { pointerId: 1 })
    await waitFor(() => expect(sent(fetch, 'POST', '/api/items/rank')).toHaveLength(1))
    expect(sent(fetch, 'POST', '/api/items/rank')[0]).toMatchObject({ path: 'tree/p/tasks/T2.md', place: 'before', other: 'tree/p/tasks/T1.md' })
    Reflect.deleteProperty(document, 'elementFromPoint')
  })

  it('previews the size a stretched bar snaps to and writes it on release', async () => {
    const { fetch } = renderAt('/roadmap/gantt?node=p')
    const handle = await screen.findByRole('button', { name: 'Resize T2' })
    fireEvent.pointerDown(handle, { pointerId: 1, clientX: 100 })
    fireEvent.pointerMove(handle, { pointerId: 1, clientX: 130 })
    expect(screen.getByTestId('stretch-preview')).toHaveTextContent('M → L · 24 h')
    fireEvent.pointerUp(handle, { pointerId: 1 })
    await waitFor(() => expect(sent(fetch, 'PATCH', '/api/items')).toEqual([{ path: 'tree/p/tasks/T2.md', changes: { size: { from: 'M', to: 'L' } } }]))
    expect(screen.queryByTestId('stretch-preview')).not.toBeInTheDocument()
  })

  it('steps a size with + and - on a focused bar', async () => {
    const { fetch } = renderAt('/roadmap/gantt?node=p')
    await screen.findByRole('button', { name: 'Resize T3' })
    screen.getByRole('button', { name: 'T3 bar' }).focus()
    await userEvent.keyboard('+')
    await waitFor(() => expect(sent(fetch, 'PATCH', '/api/items')).toEqual([{ path: 'tree/p/tasks/T3.md', changes: { size: { from: 'XS', to: 'S' } } }]))
  })

  it('refuses to resize a split story and done work', async () => {
    renderAt('/roadmap/gantt?node=p')
    await screen.findByRole('button', { name: 'Resize T2' })
    expect(screen.getByRole('button', { name: 'Resize S1: Its size is the sum of its tasks: resize those instead' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Resize S2' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /^Resize T1/ })).not.toBeInTheDocument()
  })
})
