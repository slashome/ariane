import type { NodeSummary } from '@/shared/api'
import { externalUrl, trackerOf } from './tracker'

const monday = { kind: 'monday', account: 'acme', board: 1658714524 }

describe('externalUrl', () => {
  it('links a monday id to its pulse on the board of the node', () => {
    expect(externalUrl(monday, 'monday:3151821985')).toBe(
      'https://acme.monday.com/boards/1658714524/pulses/3151821985',
    )
  })

  it('gives no link without a tracker, for another connector or without an account', () => {
    expect(externalUrl(null, 'monday:1')).toBeNull()
    expect(externalUrl(monday, 'gtasks:1')).toBeNull()
    expect(externalUrl({ kind: 'monday', board: 1 }, 'monday:1')).toBeNull()
    expect(externalUrl(monday, 'monday:')).toBeNull()
    expect(externalUrl(monday, 'monday:1/../x')).toBeNull()
  })
})

describe('trackerOf', () => {
  it('reads the effective tracker of a node, null when it has none', () => {
    const nodes: NodeSummary[] = [
      { path: 'cmc/app', items: 1, workflow: [], tracker: monday },
      { path: 'perso', items: 1, workflow: [], tracker: null },
    ]
    expect(trackerOf(nodes, 'cmc/app')).toEqual(monday)
    expect(trackerOf(nodes, 'perso')).toBeNull()
    expect(trackerOf(nodes, 'missing')).toBeNull()
  })
})
