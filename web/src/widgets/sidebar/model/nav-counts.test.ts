import type { Finding } from '@/shared/api'
import { makeItem } from '@/shared/api'
import { navCounts } from './nav-counts'

describe('navCounts', () => {
  it('counts each entry from the items in scope', () => {
    const counts = navCounts(
      [
        makeItem({ id: 'T1', size: 'S', impact: 'high' }),
        makeItem({ id: 'T2', category: 'backlog', status: 'backlog' }),
        makeItem({ id: 'T3', category: 'done', status: 'done' }),
        makeItem({ id: 'S1', kind: 'story' }),
        makeItem({ id: 'R1', kind: 'reflexion', category: null, status: 'trial' }),
        makeItem({ id: 'R2', kind: 'reflexion', category: null, status: 'settled' }),
        makeItem({ id: 'D1', kind: 'ressource', category: null, status: null }),
      ],
      [{ severity: 'error' } as Finding, { severity: 'warning' } as Finding],
    )
    expect(counts).toEqual({
      tasks: 3,
      open: 2,
      toEstimate: 1,
      backlog: 1,
      stories: 1,
      reflexions: 1,
      settled: 1,
      ressources: 1,
      errors: 1,
    })
  })
})
