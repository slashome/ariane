import { afterEach, describe, expect, it, vi } from 'vitest'
import { ApiError, addRelease, changeRelease, fetchRoadmap, removeRelease } from './client'
import type { ReleaseList, Roadmap } from './types'

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
