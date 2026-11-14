import { describe, expect, it } from 'vitest'
import type { Calendar } from '@/shared/api'
import { carrierOf, dayOffEntry, otherBlocks, refusedBlocks, requestsFor, sameDraft, savedDraft, withDayOff } from './calendar-draft'

const calendar: Calendar = {
  version: 1,
  timezone: 'Europe/Paris',
  days_off: [{ from: '2026-12-24', to: '2027-01-02' }],
  nodes: {
    cmc: {
      focus: 0.6,
      blocks: [{ days: 'mon-fri', weekdays: ['mon', 'tue', 'wed', 'thu', 'fri'], from: '09:00', to: '18:00', hours: 45 }],
      days_off: [],
    },
    'slashome/pharmakon': {
      focus: 1,
      blocks: [{ days: 'tue,thu', weekdays: ['tue', 'thu'], from: '20:00', to: '23:00', hours: 6 }],
      days_off: [{ from: '2026-08-03', to: '2026-08-03' }],
    },
  },
}

describe('the saved calendar', () => {
  it('reads the globals and the node table as a draft', () => {
    expect(savedDraft(calendar, 'slashome/pharmakon')).toEqual({
      timezone: 'Europe/Paris',
      daysOff: ['2026-12-24..2027-01-02'],
      node: { blocks: [{ days: ['tue', 'thu'], from: 1200, to: 1380 }], focus: 1, daysOff: ['2026-08-03'] },
    })
    expect(savedDraft(calendar, 'cmc/cmc-lib').node).toBeNull()
  })

  it('finds the node carrying the blocks, itself or its nearest ancestor', () => {
    expect(carrierOf(calendar, 'cmc/cmc-lib')).toBe('cmc')
    expect(carrierOf(calendar, 'slashome/pharmakon')).toBe('slashome/pharmakon')
    expect(carrierOf(calendar, 'slashome/ariane')).toBeNull()
  })

  it('greys the blocks of every other node', () => {
    expect(otherBlocks(calendar, 'slashome/pharmakon').map((entry) => entry.node)).toEqual(['cmc'])
  })
})

describe('the requests of a save', () => {
  const saved = savedDraft(calendar, 'slashome/pharmakon')
  const pharmakon = { blocks: [{ days: ['tue' as const, 'thu' as const], from: 1200, to: 1380 }], focus: 1, daysOff: ['2026-08-03'] }

  it('sends only what changed, the node before the globals', () => {
    const draft = {
      ...saved,
      timezone: 'Europe/London',
      node: { ...pharmakon, focus: 0.8, blocks: [...pharmakon.blocks, { days: ['sat' as const], from: 600, to: 780 }] },
    }
    expect(requestsFor(saved, draft, 'slashome/pharmakon')).toEqual([
      {
        kind: 'change',
        change: {
          node: 'slashome/pharmakon',
          blocks: [
            { days: 'tue,thu', from: '20:00', to: '23:00' },
            { days: 'sat', from: '10:00', to: '13:00' },
          ],
          focus: 0.8,
        },
      },
      { kind: 'change', change: { timezone: 'Europe/London' } },
    ])
  })

  it('removes the table of a node left without blocks, and unsets a focus back to 1', () => {
    expect(requestsFor(saved, { ...saved, node: { ...pharmakon, blocks: [] } }, 'slashome/pharmakon')).toEqual([
      { kind: 'remove', node: 'slashome/pharmakon' },
    ])
    const cmc = savedDraft(calendar, 'cmc')
    expect(requestsFor(cmc, { ...cmc, node: { blocks: [{ days: ['mon' as const, 'tue' as const, 'wed' as const, 'thu' as const, 'fri' as const], from: 540, to: 1080 }], focus: 1, daysOff: [] } }, 'cmc')).toEqual([{ kind: 'change', change: { node: 'cmc', focus: null } }])
  })

  it('creates the table of a node that had none', () => {
    const lib = savedDraft(calendar, 'cmc/cmc-lib')
    const draft = { ...lib, node: { blocks: [{ days: ['sat' as const], from: 600, to: 720 }], focus: 1, daysOff: [] } }
    expect(requestsFor(lib, draft, 'cmc/cmc-lib')).toEqual([
      { kind: 'change', change: { node: 'cmc/cmc-lib', blocks: [{ days: 'sat', from: '10:00', to: '12:00' }] } },
    ])
  })

  it('treats an empty own table as no table', () => {
    const lib = savedDraft(calendar, 'cmc/cmc-lib')
    expect(sameDraft(lib, { ...lib, node: { blocks: [], focus: 1, daysOff: [] } })).toBe(true)
    expect(requestsFor(lib, { ...lib, node: { blocks: [], focus: 1, daysOff: [] } }, 'cmc/cmc-lib')).toEqual([])
  })
})

describe('days off', () => {
  it('writes a day or an ordered range, and keeps the list sorted without duplicates', () => {
    expect(dayOffEntry('2026-08-03', '')).toBe('2026-08-03')
    expect(dayOffEntry('2026-08-03', '2026-08-14')).toBe('2026-08-03..2026-08-14')
    expect(dayOffEntry('2026-08-14', '2026-08-03')).toBeNull()
    expect(dayOffEntry('', '')).toBeNull()
    expect(withDayOff(['2026-12-24', '2026-05-01'], '2026-05-01')).toEqual(['2026-05-01', '2026-12-24'])
  })
})

describe('a refused overlap', () => {
  it('names the block of this node and the block of the other, whichever side the server put them', () => {
    const problems = [
      { kind: 'overlap' as const, node: 'cmc', other: 'slashome/pharmakon', block: 0, other_block: 2, message: 'overlap' },
      { kind: 'unused' as const, node: 'x', message: 'no blocks' },
    ]
    expect(refusedBlocks(problems, 'slashome/pharmakon')).toEqual({ own: [2], others: [{ node: 'cmc', block: 0 }] })
    expect(refusedBlocks(problems, 'cmc')).toEqual({ own: [0], others: [{ node: 'slashome/pharmakon', block: 2 }] })
  })
})
