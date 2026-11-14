import { makeItem, type Estimate, type Pricing } from '@/shared/api'
import { estimateBasis, estimateHours, estimateSummary, estimateTesting, formatHours, isGuessed, resizedEstimate } from './estimate'

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

describe('resizedEstimate', () => {
  const pricing: Pricing = { XS: 2, S: 4, M: 12, L: 24, XL: 48, default: 'M', test_ratio: 0.1, tester: 'others' }

  it('prices the new size of a task from its node', () => {
    const item = makeItem({ size: 'M', estimate: makeEstimate() })
    expect(resizedEstimate(item, 'L', pricing)).toEqual({
      hours: 24,
      remaining_hours: 24,
      test_hours: 2.4,
      remaining_test_hours: 2.4,
      tester: 'others',
      basis: 'size',
      size: 'L',
      parts: 1,
      guessed: 0,
    })
  })

  it('falls back on the default size, as a guess, when the size is cleared', () => {
    const resized = resizedEstimate(makeItem({ size: 'XS', estimate: makeEstimate({ hours: 2, remaining_hours: 2 }) }), null, pricing)
    expect(resized).toMatchObject({ basis: 'default', size: 'M', hours: 12, guessed: 1 })
  })

  it('keeps an unsplit story provisional', () => {
    const story = makeItem({ kind: 'story', size: 'S', estimate: makeEstimate({ basis: 'provisional', size: 'S', guessed: 1 }) })
    expect(resizedEstimate(story, 'XL', pricing)).toMatchObject({ basis: 'provisional', size: 'XL', hours: 48, guessed: 1 })
  })

  it('leaves nothing remaining on finished work, and only the testing on work in testing', () => {
    const done = makeItem({ category: 'done', size: 'M', estimate: makeEstimate({ remaining_hours: 0, remaining_test_hours: 0 }) })
    expect(resizedEstimate(done, 'S', pricing)).toMatchObject({ hours: 4, remaining_hours: 0, remaining_test_hours: 0 })
    const testing = makeItem({ category: 'in_progress', size: 'M', estimate: makeEstimate({ remaining_hours: 0 }) })
    expect(resizedEstimate(testing, 'S', pricing)).toMatchObject({ hours: 4, remaining_hours: 0, remaining_test_hours: 0.4 })
  })

  it('keeps the sum of a split story, and the estimate when the node prices nothing', () => {
    const sum = makeEstimate({ basis: 'tasks', size: null, hours: 36, parts: 3 })
    const story = makeItem({ kind: 'story', size: 'S', estimate: sum })
    expect(resizedEstimate(story, 'XL', pricing)).toBe(sum)
    const task = makeItem({ size: 'M', estimate: makeEstimate() })
    expect(resizedEstimate(task, 'L', undefined)).toBe(task.estimate)
  })
})
