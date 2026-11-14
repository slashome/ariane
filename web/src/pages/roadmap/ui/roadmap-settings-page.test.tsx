import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider } from 'react-router'
import type { CalendarOverview, NodeSummary } from '@/shared/api'
import { RoadmapSettingsPage } from './roadmap-settings-page'

const file = '/home/me/_ariane/calendar.toml'

const withCalendar: CalendarOverview = {
  file,
  present: true,
  calendar: {
    version: 1,
    timezone: 'Europe/Paris',
    days_off: [],
    nodes: {
      cmc: {
        focus: 0.6,
        blocks: [{ days: 'mon-fri', weekdays: ['mon', 'tue', 'wed', 'thu', 'fri'], from: '09:00', to: '18:00', hours: 45 }],
        days_off: [],
      },
      'slashome/pharmakon': {
        focus: 1,
        blocks: [
          { days: 'tue,thu', weekdays: ['tue', 'thu'], from: '20:00', to: '23:00', hours: 6 },
          { days: 'sat', weekdays: ['sat'], from: '10:00', to: '13:00', hours: 3 },
        ],
        days_off: [],
      },
    },
  },
  problems: [],
}

const nodes: Partial<NodeSummary>[] = [
  {
    path: 'slashome/pharmakon',
    workflow: [],
    items: 0,
    estimates: { XS: 2, S: 4, M: 10, L: 24, XL: 48, default: 'S', test_ratio: 0.25, tester: 'owner' },
    estimates_from: { XS: null, S: null, M: 'slashome', L: null, XL: null, default: 'slashome/pharmakon', test_ratio: null, tester: null },
    releases: [
      { name: '0.1', parking: false, target: '2027-01-15' },
      { name: 'plus-tard', parking: true, target: null },
    ],
    releases_from: 'slashome/pharmakon',
  },
  { path: 'cmc/cmc-lib', workflow: [], items: 0, estimates: { XS: 2, S: 4, M: 12, L: 24, XL: 48, default: 'M', test_ratio: 0.25, tester: 'owner' } },
]

type Answer = { status: number; body: unknown }

function renderAt(url: string, calendar: CalendarOverview, onPatch?: (body: unknown) => Answer) {
  const fetch = vi.fn(async (input: string, init?: RequestInit) => {
    const method = init?.method ?? 'GET'
    let answer: Answer = { status: 200, body: {} }
    if (input === '/api/calendar' && method === 'GET') {
      answer = { status: 200, body: calendar }
    } else if (input === '/api/calendar') {
      answer = onPatch?.(JSON.parse(String(init?.body))) ?? { status: 200, body: calendar }
    } else if (input.startsWith('/api/nodes')) {
      answer = { status: 200, body: nodes }
    } else if (input === '/api/estimates') {
      answer = { status: 200, body: { node: 'slashome/pharmakon', estimates: nodes[0]?.estimates, estimates_from: nodes[0]?.estimates_from } }
    }
    return new Response(JSON.stringify(answer.body), { status: answer.status, headers: { 'Content-Type': 'application/json' } })
  })
  vi.stubGlobal('fetch', fetch)
  const router = createMemoryRouter([{ path: '/roadmap/settings', element: <RoadmapSettingsPage /> }], { initialEntries: [url] })
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  )
  return { fetch, router }
}

function sent(fetch: ReturnType<typeof vi.fn>, method: string, url: string): unknown[] {
  return fetch.mock.calls
    .filter(([input, init]) => input === url && (init as RequestInit | undefined)?.method === method)
    .map(([, init]) => JSON.parse(String((init as RequestInit).body)) as unknown)
}

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('the roadmap settings', () => {
  it('names the two blocks and the file each one writes', async () => {
    renderAt('/roadmap/settings?node=slashome/pharmakon', withCalendar)
    expect(await screen.findByRole('region', { name: /Personal/ })).toBeInTheDocument()
    expect(await screen.findByTestId('personal-file')).toHaveTextContent(file)
    expect(screen.getByRole('region', { name: /Shared with the team/ })).toBeInTheDocument()
    expect(screen.getByTestId('shared-file')).toHaveTextContent('tree/slashome/pharmakon/node.toml')
  })

  it('creates the calendar from an empty state', async () => {
    const { fetch } = renderAt('/roadmap/settings?node=slashome/pharmakon', { file, present: false, calendar: null, problems: [] })
    await userEvent.click(await screen.findByRole('button', { name: 'Create my calendar' }))
    await waitFor(() => expect(sent(fetch, 'PATCH', '/api/calendar')).toHaveLength(1))
    expect(sent(fetch, 'PATCH', '/api/calendar')[0]).toEqual({ timezone: expect.any(String) })
  })

  it('draws the blocks of the node and greys the others, with the weekly focused hours', async () => {
    renderAt('/roadmap/settings?node=slashome/pharmakon', withCalendar)
    expect(await screen.findAllByTestId('own-block')).toHaveLength(3)
    expect(screen.getAllByTestId('other-block')).toHaveLength(5)
    expect(screen.getAllByRole('img', { name: 'cmc: mon-fri 09:00–18:00' })).toHaveLength(5)
    expect(screen.getByTestId('focus-summary')).toHaveTextContent('9 h of blocks × 1 = 9 h of focused work per week')
  })

  it('draws a block by sweeping the week, and deletes one from the keyboard', async () => {
    renderAt('/roadmap/settings?node=slashome/pharmakon', withCalendar)
    await screen.findAllByTestId('own-block')
    const area = screen.getByTestId('week-area')
    area.getBoundingClientRect = () => ({ left: 0, top: 0, width: 700, height: 480, right: 700, bottom: 480, x: 0, y: 0, toJSON: () => ({}) })
    fireEvent.pointerDown(area, { button: 0, clientX: 550, clientY: 14 * 20 + 3 })
    fireEvent.pointerMove(area, { clientX: 650, clientY: 16 * 20 - 2 })
    fireEvent.pointerUp(area, { clientX: 650, clientY: 16 * 20 - 2 })
    expect(screen.getAllByRole('button', { name: 'sat,sun 14:00–16:00, block 3' })).toHaveLength(2)
    expect(screen.getByTestId('focus-summary')).toHaveTextContent('13 h of blocks')
    fireEvent.keyDown(screen.getByRole('button', { name: 'sat 10:00–13:00, block 2' }), { key: 'Delete' })
    expect(screen.getAllByTestId('own-block')).toHaveLength(4)
    expect(screen.getByRole('region', { name: 'Unsaved calendar' })).toBeInTheDocument()
  })

  it('says where an inherited node takes its hours, and lets it have its own', async () => {
    const { router } = renderAt('/roadmap/settings?node=cmc/cmc-lib', withCalendar)
    const notice = await screen.findByRole('note', { name: 'Inherited hours' })
    expect(notice).toHaveTextContent('Your hours for cmc/cmc-lib come from cmc — shared with its sub-nodes.')
    expect(screen.getByTestId('focus-summary')).toHaveTextContent('45 h of blocks × 0.6 = 27 h of focused work per week')
    await userEvent.click(within(notice).getByRole('button', { name: 'Give cmc/cmc-lib its own blocks' }))
    expect(screen.getByRole('slider')).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: "Use cmc's hours" }))
    await userEvent.click(within(screen.getByRole('note', { name: 'Inherited hours' })).getByRole('link', { name: 'Open cmc' }))
    expect(new URLSearchParams(router.state.location.search).get('node')).toBe('cmc')
  })

  it('shows the save bar only while the draft differs, and saves on demand', async () => {
    const { fetch } = renderAt('/roadmap/settings?node=slashome/pharmakon', withCalendar)
    const focus = await screen.findByRole('spinbutton', { name: 'Focus value' })
    expect(screen.queryByRole('region', { name: 'Unsaved calendar' })).toBeNull()
    fireEvent.change(focus, { target: { value: '0.8' } })
    expect(screen.getByTestId('focus-summary')).toHaveTextContent('9 h of blocks × 0.8 = 7.2 h of focused work per week')
    const bar = screen.getByRole('region', { name: 'Unsaved calendar' })
    await userEvent.click(within(bar).getByRole('button', { name: 'Discard' }))
    expect(screen.queryByRole('region', { name: 'Unsaved calendar' })).toBeNull()
    fireEvent.change(screen.getByRole('spinbutton', { name: 'Focus value' }), { target: { value: '0.8' } })
    expect(sent(fetch, 'PATCH', '/api/calendar')).toHaveLength(0)
    await userEvent.click(within(screen.getByRole('region', { name: 'Unsaved calendar' })).getByRole('button', { name: 'Save' }))
    await waitFor(() => expect(sent(fetch, 'PATCH', '/api/calendar')).toEqual([{ node: 'slashome/pharmakon', focus: 0.8 }]))
  })

  it('keeps Save off while the browser sees an overlap, with the reason on hover', async () => {
    const { fetch } = renderAt('/roadmap/settings?node=slashome/pharmakon', withCalendar)
    const blocks = await screen.findByRole('list', { name: 'Blocks' })
    fireEvent.change(within(blocks).getByRole('combobox', { name: 'Start of block 2' }), { target: { value: String(9 * 60) } })
    await userEvent.click(within(within(blocks).getByRole('group', { name: 'Days of block 2' })).getByRole('button', { name: 'mo' }))
    expect(screen.getByRole('alert')).toHaveTextContent('overlaps cmc mon-fri 09:00–18:00')
    expect(within(blocks).getAllByRole('listitem')[1]?.dataset.overlap).toBe('true')
    const bar = screen.getByRole('region', { name: 'Unsaved calendar' })
    expect(within(bar).getByRole('button', { name: 'Save' })).toBeDisabled()
    const blocked = within(bar).getByTestId('save-blocked')
    expect(blocked).toHaveAccessibleName('Cannot save: mon,sat 09:00–13:00 overlaps cmc mon-fri 09:00–18:00')
    await userEvent.hover(blocked)
    expect(await screen.findByRole('tooltip')).toHaveTextContent('overlaps cmc mon-fri 09:00–18:00')
    expect(sent(fetch, 'PATCH', '/api/calendar')).toHaveLength(0)
    await userEvent.click(within(within(blocks).getByRole('group', { name: 'Days of block 2' })).getByRole('button', { name: 'mo' }))
    expect(within(screen.getByRole('region', { name: 'Unsaved calendar' })).getByRole('button', { name: 'Save' })).toBeEnabled()
  })

  it('marks the blocks the server refuses apart from the overlaps the browser sees', async () => {
    const { fetch } = renderAt('/roadmap/settings?node=slashome/pharmakon', withCalendar, () => ({
      status: 422,
      body: {
        error: 'calendar.toml not written: the blocks overlap',
        problems: [{ kind: 'overlap', node: 'cmc', other: 'slashome/pharmakon', block: 0, other_block: 1, message: 'overlap' }],
      },
    }))
    const blocks = await screen.findByRole('list', { name: 'Blocks' })
    fireEvent.change(within(blocks).getByRole('combobox', { name: 'Start of block 2' }), { target: { value: String(11 * 60) } })
    await userEvent.click(within(screen.getByRole('region', { name: 'Unsaved calendar' })).getByRole('button', { name: 'Save' }))
    await waitFor(() => expect(sent(fetch, 'PATCH', '/api/calendar')).toHaveLength(1))
    expect(await screen.findByRole('alert', { name: 'Save refused' })).toHaveTextContent('The server refused the save: calendar.toml not written: the blocks overlap')
    const refused = screen.getAllByTestId('own-block').filter((block) => block.dataset.refused)
    expect(refused).not.toHaveLength(0)
    expect(refused.every((block) => block.getAttribute('aria-label')?.endsWith('block 2') && !block.dataset.overlap)).toBe(true)
    expect(screen.getAllByTestId('other-block').filter((block) => block.dataset.refused)).toHaveLength(5)
    expect(within(blocks).getAllByRole('listitem')[1]?.dataset.refused).toBe('true')
    expect(screen.getByRole('region', { name: 'Unsaved calendar' })).toBeInTheDocument()
  })

  it('shows where each estimate comes from, writes this node and resets its own keys', async () => {
    const { fetch } = renderAt('/roadmap/settings?node=slashome/pharmakon', withCalendar)
    expect(await screen.findByTestId('origin-M')).toHaveTextContent('from slashome')
    expect(screen.getByTestId('origin-default')).toHaveTextContent('own')
    expect(screen.getByTestId('origin-XL')).toHaveTextContent('default')
    expect(screen.queryByRole('button', { name: 'Reset M' })).toBeNull()
    const xl = screen.getByRole('spinbutton', { name: 'XL' })
    await userEvent.clear(xl)
    await userEvent.type(xl, '40{Enter}')
    await userEvent.click(screen.getByRole('button', { name: 'Reset Default size' }))
    await waitFor(() =>
      expect(sent(fetch, 'PATCH', '/api/estimates')).toEqual([
        { node: 'slashome/pharmakon', estimates: { XL: 40 } },
        { node: 'slashome/pharmakon', estimates: { default: null } },
      ]),
    )
  })

  it('lists the releases in order and links to the Releases view', async () => {
    renderAt('/roadmap/settings?node=slashome/pharmakon', withCalendar)
    const releases = await screen.findByRole('list', { name: 'Releases' })
    expect(within(releases).getAllByRole('listitem').map((entry) => entry.textContent)).toEqual(['0.1 · 2027-01-15', 'plus-tard · parking'])
    expect(screen.getByRole('link', { name: 'Plan them in Releases' })).toHaveAttribute('href', '/roadmap/releases?node=slashome%2Fpharmakon')
  })
})
