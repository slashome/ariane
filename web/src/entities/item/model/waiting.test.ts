import { makeItem } from '@/shared/api'
import { daysBetween, today, waitingNames, waitingSummary } from './waiting'

const people = { keyvan: { name: 'Keyvan Coen', monday: 1 }, doryan: { name: 'Doryan' } }

describe('waiting', () => {
  it('names the people from the directory, an unknown handle as is', () => {
    expect(waitingNames(makeItem({ waiting_for: ['keyvan', 'ghost'] }), people)).toEqual(['Keyvan Coen', 'ghost'])
  })

  it('counts the days of waiting', () => {
    expect(daysBetween('2026-10-20', '2026-10-23')).toBe(3)
    expect(daysBetween(null, '2026-10-23')).toBeNull()
    expect(daysBetween('nope', '2026-10-23')).toBeNull()
    expect(today(new Date(2026, 0, 5))).toBe('2026-01-05')
  })

  it('sums it up for a card', () => {
    const item = makeItem({ waiting_for: ['keyvan', 'doryan'], waiting_since: '2026-10-21' })
    expect(waitingSummary(item, people, '2026-10-23')).toBe('Keyvan Coen, Doryan · 2 d')
    expect(waitingSummary(makeItem({ waiting_for: ['doryan'] }), people, '2026-10-23')).toBe('Doryan')
  })
})
