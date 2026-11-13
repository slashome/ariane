import { afterEach, describe, expect, it, vi } from 'vitest'
import { ApiError, fetchRoadmap } from './client'
import type { Roadmap } from './types'

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
