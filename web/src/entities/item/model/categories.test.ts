import type { Item } from '@/shared/api'
import { changesFor, isClosed, isWorkItem } from './categories'

const item: Item = {
  node: 'game',
  id: 'T001',
  title: 'Forge',
  kind: 'task',
  archived: false,
  status: 'a-faire',
  category: 'todo',
  size: 'M',
  impact: null,
  priority: null,
  created: null,
  reference: null,
  type: null,
  personas: [],
  release: null,
  milestone: null,
  parent: null,
  depends_on: [],
  external: [],
  branches: [],
  pull_requests: [],
  waiting_for: [],
  waiting_on: null,
  waiting_since: null,
  version: 1,
  path: 'tree/game/tasks/T001.md',
}

describe('item model', () => {
  it('records the current value as `from` for every changed field only', () => {
    expect(changesFor(item, { size: 'XS', impact: 'high' })).toEqual({
      size: { from: 'M', to: 'XS' },
      impact: { from: null, to: 'high' },
    })
    expect(changesFor(item, { size: 'M' })).toEqual({})
  })

  it('compares lists by value and treats an empty list as no value', () => {
    const waiting = { ...item, waiting_for: ['keyvan'] }
    expect(changesFor(waiting, { waiting_for: ['keyvan'] })).toEqual({})
    expect(changesFor(waiting, { waiting_for: ['keyvan', 'doryan'] })).toEqual({
      waiting_for: { from: ['keyvan'], to: ['keyvan', 'doryan'] },
    })
    expect(changesFor(item, { waiting_for: [] })).toEqual({})
  })

  it('tells closed categories and work items apart', () => {
    expect(isClosed('done')).toBe(true)
    expect(isClosed('dropped')).toBe(true)
    expect(isClosed('blocked')).toBe(false)
    expect(isWorkItem(item)).toBe(true)
    expect(isWorkItem({ ...item, kind: 'reflexion' })).toBe(false)
  })
})
