import { describe, expect, it } from 'vitest'
import {
  describeOverlap,
  drawnBlock,
  focusSummary,
  formatDays,
  fromCalendarBlock,
  movedBlock,
  overlapsOf,
  parseDays,
  resizedBlock,
  saveBlocker,
  snap,
  toBlockSpec,
  toggledDay,
  weeklyHours,
  type Block,
} from './week-grid'

const evening: Block = { days: ['tue', 'thu'], from: 20 * 60, to: 23 * 60 }
const office: Block = { days: ['mon', 'tue', 'wed', 'thu', 'fri'], from: 9 * 60, to: 18 * 60 }

describe('days and times', () => {
  it('reads and writes days as ranges and lists', () => {
    expect(parseDays('mon,wed-fri')).toEqual(['mon', 'wed', 'thu', 'fri'])
    expect(formatDays(['mon', 'tue', 'wed', 'thu', 'fri'])).toBe('mon-fri')
    expect(formatDays(['tue', 'thu'])).toBe('tue,thu')
    expect(formatDays(['sat', 'sun'])).toBe('sat,sun')
    expect(formatDays(['mon', 'wed', 'thu', 'fri'])).toBe('mon,wed-fri')
  })

  it('round-trips a block of the server', () => {
    const block = fromCalendarBlock({ days: 'mon-fri', weekdays: ['mon', 'tue', 'wed', 'thu', 'fri'], from: '09:00', to: '18:00', hours: 45 })
    expect(block).toEqual(office)
    expect(toBlockSpec({ ...evening, to: 24 * 60 })).toEqual({ days: 'tue,thu', from: '20:00', to: '24:00' })
  })

  it('snaps to half hours within the day', () => {
    expect(snap(9 * 60 + 14)).toBe(9 * 60)
    expect(snap(9 * 60 + 16)).toBe(9 * 60 + 30)
    expect(snap(-20)).toBe(0)
    expect(snap(25 * 60)).toBe(24 * 60)
  })
})

describe('the grid gestures', () => {
  it('draws a block over the days and hours swept, whatever the direction', () => {
    expect(drawnBlock({ day: 4, minute: 18 * 60 + 5 }, { day: 0, minute: 8 * 60 + 50 })).toEqual(office)
  })

  it('draws at least half an hour, even at the end of the day', () => {
    expect(drawnBlock({ day: 5, minute: 600 }, { day: 5, minute: 605 })).toEqual({ days: ['sat'], from: 600, to: 630 })
    expect(drawnBlock({ day: 6, minute: 1440 }, { day: 6, minute: 1440 })).toEqual({ days: ['sun'], from: 1410, to: 1440 })
  })

  it('moves a block in time and across days, keeping it inside the week', () => {
    expect(movedBlock(evening, 1, 40)).toEqual({ days: ['wed', 'fri'], from: 20 * 60 + 30, to: 23 * 60 + 30 })
    expect(movedBlock(evening, 5, 3 * 60)).toEqual({ days: ['fri', 'sun'], from: 21 * 60, to: 24 * 60 })
    expect(movedBlock(office, -2, -10 * 60)).toEqual({ ...office, from: 0, to: 9 * 60 })
  })

  it('resizes either edge to the snap, never under half an hour', () => {
    expect(resizedBlock(evening, 'to', 23 * 60 + 50)).toEqual({ ...evening, to: 24 * 60 })
    expect(resizedBlock(evening, 'from', 19 * 60 + 10)).toEqual({ ...evening, from: 19 * 60 })
    expect(resizedBlock(evening, 'from', 23 * 60 + 30)).toEqual({ ...evening, from: 22 * 60 + 30 })
  })

  it('toggles a day, and the last day off removes the block', () => {
    expect(toggledDay(evening, 'sat')).toEqual({ ...evening, days: ['tue', 'thu', 'sat'] })
    expect(toggledDay({ ...evening, days: ['tue'] }, 'tue')).toBeNull()
  })
})

describe('overlaps', () => {
  it('finds the blocks sharing time on a day, within the node and with others', () => {
    const own: Block[] = [evening, { days: ['thu'], from: 22 * 60, to: 24 * 60 }, { days: ['wed'], from: 17 * 60, to: 19 * 60 }]
    expect(overlapsOf(own, [{ node: 'cmc', blocks: [office] }])).toEqual([
      { block: 0, other: null, otherBlock: 1 },
      { block: 2, other: 'cmc', otherBlock: 0 },
    ])
  })

  it('lets blocks touch', () => {
    expect(overlapsOf([{ days: ['mon'], from: 18 * 60, to: 20 * 60 }], [{ node: 'cmc', blocks: [office] }])).toEqual([])
  })

  it('names both blocks of an overlap, and the other node when there is one', () => {
    const own: Block[] = [evening, { days: ['thu'], from: 22 * 60, to: 24 * 60 }, { days: ['wed'], from: 17 * 60, to: 19 * 60 }]
    const others = [{ node: 'cmc', blocks: [office] }]
    expect(describeOverlap({ block: 0, other: null, otherBlock: 1 }, own, others)).toBe('tue,thu 20:00–23:00 overlaps thu 22:00–24:00')
    expect(describeOverlap({ block: 2, other: 'cmc', otherBlock: 0 }, own, others)).toBe('wed 17:00–19:00 overlaps cmc mon-fri 09:00–18:00')
  })

  it('blocks the save while any overlap remains, with every reason', () => {
    const others = [{ node: 'cmc', blocks: [office] }]
    expect(saveBlocker([evening], others)).toBeNull()
    expect(saveBlocker([evening, { days: ['wed'], from: 17 * 60, to: 19 * 60 }, { days: ['thu'], from: 22 * 60, to: 24 * 60 }], others)).toBe(
      'Cannot save: tue,thu 20:00–23:00 overlaps thu 22:00–24:00; wed 17:00–19:00 overlaps cmc mon-fri 09:00–18:00',
    )
  })
})

describe('weekly hours', () => {
  it('sums the blocks times their days, and the focused share', () => {
    expect(weeklyHours([office])).toBe(45)
    expect(weeklyHours([evening, { days: ['sat'], from: 600, to: 780 }])).toBe(9)
    expect(focusSummary(45, 0.6)).toBe('45 h of blocks × 0.6 = 27 h of focused work per week')
  })
})
