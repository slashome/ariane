import { makeScale, pastHoursPerDay, ticks } from './scale'

const now = Date.parse('2026-11-13T12:00:00Z')

describe('makeScale', () => {
  it('draws the past of an undated forecast in days as wide as a day of focused hours', () => {
    const scale = makeScale(2, false)
    expect(scale.x(10)).toBe(20)
    expect(scale.x(-24)).toBe(-pastHoursPerDay * 2)
    expect(scale.time(scale.x(-48))).toBe(-48)
  })

  it('keeps one scale for a dated forecast', () => {
    const scale = makeScale(1, true)
    expect(scale.x(-24)).toBe(-24)
  })
})

describe('ticks', () => {
  it('counts focused hours from now on an undated axis, dates in the past', () => {
    const scale = makeScale(2, false)
    const marks = ticks(scale, false, now, -200, 400)
    const future = marks.filter((tick) => tick.x >= 0).map((tick) => tick.label)
    expect(future.slice(0, 3)).toEqual(['+50 h', '+100 h', '+150 h'])
    expect(marks.filter((tick) => tick.x < 0).every((tick) => !tick.label.startsWith('+'))).toBe(true)
  })

  it('steps by day, week or month with the zoom on a dated axis', () => {
    const daily = ticks(makeScale(2, true), true, now, 0, 48 * 7)
    expect(daily).toHaveLength(7)
    const weekly = ticks(makeScale(0.5, true), true, now, 0, 12 * 28)
    expect(weekly.length).toBeLessThanOrEqual(5)
    const monthly = ticks(makeScale(0.25, true), true, now, 0, 6 * 24 * 90)
    expect(monthly.every((tick) => /\d{4}/.test(tick.label))).toBe(true)
  })
})
