import { makeItem } from '@/shared/api'
import { buildLanes, journal } from './reflexions'

const reflexion = (id: string, status: string, updated: string | null) =>
  makeItem({ id, kind: 'reflexion', status, category: null, updated })

describe('reflexion lanes', () => {
  it('put each reflexion in the lane of its status, the latest change first when asked', () => {
    const lanes = buildLanes(
      [reflexion('R1', 'open', '2026-01-01'), reflexion('R2', 'trial', null), reflexion('R3', 'open', '2026-03-01')],
      ['open', 'trial', 'settled'],
      'recent',
    )
    expect(lanes.map((lane) => [lane.status, lane.items.map((item) => item.id)])).toEqual([
      ['open', ['R3', 'R1']],
      ['trial', ['R2']],
      ['settled', []],
    ])
  })
})

describe('reflexion lanes in manual order', () => {
  it('follow the rank of each reflexion, then its id', () => {
    const lanes = buildLanes(
      [reflexion('R1', 'open', '2026-03-01'), { ...reflexion('R2', 'open', null), rank: 1000 }, reflexion('R3', 'open', '2026-01-01')],
      ['open'],
    )
    expect(lanes.map((lane) => [lane.id, lane.items.map((item) => item.id)])).toEqual([['lane:open', ['R2', 'R1', 'R3']]])
  })
})

describe('journal', () => {
  it('lists settled reflexions by month of their latest change, newest first, undated last', () => {
    const groups = journal([
      reflexion('R1', 'settled', '2026-08-03'),
      reflexion('R2', 'settled', '2026-10-21'),
      reflexion('R3', 'open', '2026-10-22'),
      reflexion('R4', 'settled', null),
      reflexion('R5', 'settled', '2026-10-02'),
    ])
    expect(groups.map((group) => [group.label, group.items.map((item) => item.id)])).toEqual([
      ['October 2026', ['R2', 'R5']],
      ['August 2026', ['R1']],
      ['Undated', ['R4']],
    ])
  })
})
