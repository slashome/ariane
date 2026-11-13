import type { Estimate } from '@/shared/api'
import { estimateBasis, estimateHours, estimateSummary, estimateTesting, formatHours, isGuessed } from './estimate'

function makeEstimate(overrides: Partial<Estimate> = {}): Estimate {
  return {
    hours: 12,
    remaining_hours: 12,
    test_hours: 3,
    remaining_test_hours: 3,
    tester: 'owner',
    basis: 'size',
    size: 'M',
    parts: 1,
    guessed: 0,
    ...overrides,
  }
}

describe('estimate', () => {
  it('writes hours without trailing decimals', () => {
    expect(formatHours(12)).toBe('12 h')
    expect(formatHours(1.2000000000000002)).toBe('1.2 h')
  })

  it('marks a default, provisional or partly guessed estimate as a guess', () => {
    expect(isGuessed(makeEstimate())).toBe(false)
    expect(isGuessed(makeEstimate({ basis: 'default' }))).toBe(true)
    expect(isGuessed(makeEstimate({ basis: 'provisional', size: 'L' }))).toBe(true)
    expect(isGuessed(makeEstimate({ basis: 'tasks', size: null, parts: 3, guessed: 0 }))).toBe(false)
    expect(isGuessed(makeEstimate({ basis: 'tasks', size: null, parts: 3, guessed: 1 }))).toBe(true)
    expect(estimateHours(makeEstimate({ basis: 'default' }))).toBe('~12 h')
    expect(estimateHours(makeEstimate())).toBe('12 h')
  })

  it('says where the hours come from and who tests', () => {
    expect(estimateBasis(makeEstimate({ basis: 'tasks', size: null, parts: 3, guessed: 2 }))).toBe(
      'Sum of 3 parts, 2 guessed from a default or provisional size',
    )
    expect(estimateBasis(makeEstimate({ basis: 'tasks', size: null, hours: 0, parts: 0 }))).toBe(
      'Every part is dropped: nothing left to do',
    )
    expect(estimateBasis(makeEstimate({ basis: 'default' }))).toBe('No size: counted as the default size M')
    expect(estimateTesting(makeEstimate({ tester: 'others' }))).toBe('+3 h of testing by others, as a delay')
    expect(estimateTesting(makeEstimate({ test_hours: 0 }))).toBeNull()
    expect(estimateSummary(makeEstimate({ remaining_hours: 4 }))).toBe(
      '12 h of focused work\nSize M\n+3 h of testing by the owner\n4 h remaining',
    )
  })
})
