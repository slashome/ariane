import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  ApiError,
  addRelease,
  changeCalendar,
  changeEstimates,
  changeRelease,
  fetchRoadmap,
  removeCalendarNode,
  removeRelease,
} from './client'
import type { CalendarOverview, CalendarRefusal, EstimatesReport, ReleaseList, Roadmap } from './types'

const roadmap: Roadmap = {
  node: 'slashome/ariane',
  carrier: null,
  timezone: null,
  now: '2026-11-16T07:00:00Z',
  dated: false,
  hours: 2,
  test_hours: 0,
  releases: [
    {
      name: '0.2',
      declared_by: 'slashome/ariane',
      target: null,
      hours: 2,
      test_hours: 0,
      span: { start: { hours: 0, at: null }, end: { hours: 2, at: null } },
      closed: null,
      done: 0,
      open: 1,
      epics: [],
    },
  ],
  unscheduled: [],
  warnings: [{ kind: 'unsized', count: 1, items: [{ reference: 'slashome/ariane#T1', detail: null }] }],
}

function answering(status: number, body: unknown) {
  const fetch = vi.fn<(url: string) => Promise<Response>>(async () => new Response(JSON.stringify(body), { status }))
  vi.stubGlobal('fetch', fetch)
  return fetch
}

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('fetchRoadmap', () => {
  it('asks the roadmap of the node and returns the forecast as served', async () => {
    const fetch = answering(200, roadmap)
    const answer = await fetchRoadmap('slashome/ariane')
    expect(fetch.mock.calls[0]?.[0]).toBe('/api/roadmap?node=slashome%2Fariane')
    expect(answer).toEqual(roadmap)
    expect(answer.releases[0]?.span?.end.hours).toBe(2)
  })

  it('fails with the message of the server for an unknown node', async () => {
    answering(404, { error: '`nope` is not a node of the tree' })
    await expect(fetchRoadmap('nope')).rejects.toThrow(ApiError)
    await expect(fetchRoadmap('nope')).rejects.toThrow('`nope` is not a node of the tree')
  })
})

const list: ReleaseList = {
  node: 'p',
  releases_from: 'p',
  releases: [
    { name: '0.9', parking: false, target: '2027-01-15' },
    { name: 'plus-tard', parking: true, target: null },
  ],
  rewritten: [],
}

function receiving(status: number, body: unknown) {
  const fetch = vi.fn<(url: string, init?: RequestInit) => Promise<Response>>(
    async () => new Response(JSON.stringify(body), { status }),
  )
  vi.stubGlobal('fetch', fetch)
  return fetch
}

describe('release writes', () => {
  it('sends each change to /api/releases with its method and returns the new list', async () => {
    const fetch = receiving(200, list)
    const change = { node: 'p', name: '0.9', place: 'top' as const, target: '2027-01-15' }
    expect(await addRelease(change)).toEqual(list)
    await changeRelease({ node: 'p', name: '0.9', rename_to: 'mango' })
    await removeRelease({ node: 'p', name: 'mango' })
    const sent = fetch.mock.calls.map(([url, init]) => [url, init?.method, JSON.parse(String(init?.body))])
    expect(sent).toEqual([
      ['/api/releases', 'POST', change],
      ['/api/releases', 'PATCH', { node: 'p', name: '0.9', rename_to: 'mango' }],
      ['/api/releases', 'DELETE', { node: 'p', name: 'mango' }],
    ])
  })

  it('keeps the items that still name a release and the node that declares an inherited list', async () => {
    receiving(409, { error: '`1.0` is still the release of p#T1', items: ['p#T1'] })
    const used = await removeRelease({ node: 'p', name: '1.0' }).catch((error: unknown) => error)
    expect(used).toBeInstanceOf(ApiError)
    expect((used as ApiError).status).toBe(409)
    expect((used as ApiError).body).toEqual({ error: '`1.0` is still the release of p#T1', items: ['p#T1'] })
    receiving(422, { error: '`p/c` inherits its releases from `p`: change them there', declared_in: 'p' })
    const inherited = await addRelease({ node: 'p/c', name: '2.0' }).catch((error: unknown) => error)
    expect((inherited as ApiError).message).toBe('`p/c` inherits its releases from `p`: change them there')
    expect((inherited as ApiError).body).toMatchObject({ declared_in: 'p' })
  })
})

const overview: CalendarOverview = {
  file: '/home/me/_ariane/calendar.toml',
  present: true,
  calendar: {
    version: 1,
    timezone: 'Europe/Paris',
    days_off: [],
    nodes: {
      p: {
        focus: 0.5,
        blocks: [{ days: 'mon-fri', weekdays: ['mon', 'tue', 'wed', 'thu', 'fri'], from: '09:00', to: '12:00', hours: 15 }],
        days_off: [],
      },
    },
  },
  problems: [],
}

describe('calendar writes', () => {
  it('sends a node change, a global change and a removal to /api/calendar', async () => {
    const fetch = receiving(200, overview)
    const change = { node: 'p', blocks: [{ days: 'mon-fri', from: '09:00', to: '12:00' }], focus: null }
    expect(await changeCalendar(change)).toEqual(overview)
    await changeCalendar({ timezone: 'Europe/Paris', days_off: [] })
    await removeCalendarNode('p')
    const sent = fetch.mock.calls.map(([url, init]) => [url, init?.method, JSON.parse(String(init?.body))])
    expect(sent).toEqual([
      ['/api/calendar', 'PATCH', change],
      ['/api/calendar', 'PATCH', { timezone: 'Europe/Paris', days_off: [] }],
      ['/api/calendar', 'DELETE', { node: 'p' }],
    ])
  })

  it('keeps the problems of a refused change, with the blocks that overlap', async () => {
    const refusal: CalendarRefusal = {
      error: 'calendar.toml not written: the blocks overlap',
      problems: [{ kind: 'overlap', node: 'p', other: 'q', block: 1, other_block: 0, message: 'the blocks overlap' }],
    }
    receiving(422, refusal)
    const refused = await changeCalendar({ node: 'q', blocks: [{ days: 'wed', from: '11:00', to: '13:00' }] }).catch(
      (error: unknown) => error,
    )
    expect(refused).toBeInstanceOf(ApiError)
    expect((refused as ApiError).status).toBe(422)
    expect((refused as ApiError).message).toBe('calendar.toml not written: the blocks overlap')
    expect(((refused as ApiError).body as CalendarRefusal).problems?.[0]).toMatchObject({ block: 1, other_block: 0 })
  })
})

describe('estimates writes', () => {
  it('sends the keys to set or remove and returns where each value comes from', async () => {
    const report: EstimatesReport = {
      node: 'p/c',
      estimates: { XS: 2, S: 4, M: 10, L: 24, XL: 48, default: 'S', test_ratio: 0.25, tester: 'owner' },
      estimates_from: { XS: null, S: null, M: 'p', L: null, XL: null, default: 'p/c', test_ratio: null, tester: null },
    }
    const fetch = receiving(200, report)
    const change = { node: 'p/c', estimates: { default: 'S', XL: null } }
    expect(await changeEstimates(change)).toEqual(report)
    const [url, init] = fetch.mock.calls[0] ?? []
    expect([url, init?.method, JSON.parse(String(init?.body))]).toEqual(['/api/estimates', 'PATCH', change])
  })
})
