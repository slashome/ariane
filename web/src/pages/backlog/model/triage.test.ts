import { makeItem } from '@/shared/api'
import { cycle, missingFields, sortBacklog } from './triage'

const levels = ['low', 'medium', 'high']
const sizes = ['XS', 'S', 'M', 'L', 'XL']

describe('sortBacklog', () => {
  const items = [
    makeItem({ id: 'T1', priority: 'low', impact: 'high', size: 'XS' }),
    makeItem({ id: 'T2', priority: 'high', impact: 'low', size: 'XL' }),
    makeItem({ id: 'T3', priority: 'high', impact: 'high', size: 'L' }),
    makeItem({ id: 'T4', priority: 'high', impact: 'high', size: 'S' }),
    makeItem({ id: 'T5', created: '2026-01-01' }),
  ]
  const order = (sort: Parameters<typeof sortBacklog>[1]) => sortBacklog(items, sort, levels, sizes).map((item) => item.id)

  it('triages by priority, then impact, then the smallest size, unset last', () => {
    expect(order('triage')).toEqual(['T4', 'T3', 'T2', 'T1', 'T5'])
  })

  it('sorts by one field, ties broken by id', () => {
    expect(order('impact')).toEqual(['T1', 'T3', 'T4', 'T2', 'T5'])
    expect(order('size')).toEqual(['T1', 'T4', 'T3', 'T2', 'T5'])
    expect(order('created')[0]).toBe('T5')
  })
})

describe('triage helpers', () => {
  it('cycles a field through its vocabulary and back to unset', () => {
    expect(cycle(levels, null)).toBe('low')
    expect(cycle(levels, 'medium')).toBe('high')
    expect(cycle(levels, 'high')).toBeNull()
  })

  it('names the fields still to decide', () => {
    expect(missingFields(makeItem({ priority: 'low' }))).toEqual(['impact', 'size'])
  })
})
