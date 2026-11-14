import { describe, expect, it } from 'vitest'
import type { Pricing } from '@/shared/api'
import { estimateEdit, nodeTomlPath, originOf, resetChange } from './estimates'

const pricing: Pricing = { XS: 2, S: 4, M: 12, L: 24, XL: 48, default: 'M', test_ratio: 0.25, tester: 'owner' }

describe('where an estimate comes from', () => {
  it('says own, the ancestor it is inherited from, or the default', () => {
    expect(originOf('cmc/lib', 'cmc/lib')).toEqual({ kind: 'own', label: 'own' })
    expect(originOf('cmc/lib', 'cmc')).toEqual({ kind: 'inherited', label: 'from cmc' })
    expect(originOf('cmc/lib', '')).toEqual({ kind: 'inherited', label: 'from (home)' })
    expect(originOf('cmc/lib', null)).toEqual({ kind: 'default', label: 'default' })
  })

  it('names the node.toml it writes', () => {
    expect(nodeTomlPath('slashome/pharmakon')).toBe('tree/slashome/pharmakon/node.toml')
    expect(nodeTomlPath('')).toBe('tree/node.toml')
  })
})

describe('an edited estimate', () => {
  it('writes one key of this node, typed', () => {
    expect(estimateEdit('p', 'M', ' 10 ', pricing)).toEqual({ change: { node: 'p', estimates: { M: 10 } } })
    expect(estimateEdit('p', 'test_ratio', '0', pricing)).toEqual({ change: { node: 'p', estimates: { test_ratio: 0 } } })
    expect(estimateEdit('p', 'default', 'S', pricing)).toEqual({ change: { node: 'p', estimates: { default: 'S' } } })
    expect(estimateEdit('p', 'tester', 'others', pricing)).toEqual({ change: { node: 'p', estimates: { tester: 'others' } } })
  })

  it('sends nothing for an unchanged value and refuses a wrong one', () => {
    expect(estimateEdit('p', 'M', '12', pricing)).toBeNull()
    expect(estimateEdit('p', 'M', '0', pricing)).toEqual({ error: 'M is a number of hours above zero' })
    expect(estimateEdit('p', 'test_ratio', '-1', pricing)).toEqual({ error: 'The test ratio is zero or above' })
    expect(estimateEdit('p', 'XL', 'many', pricing)).toEqual({ error: 'XL is a number' })
    expect(estimateEdit('p', 'default', 'XXL', pricing)).toMatchObject({ error: expect.stringContaining('XS, S, M, L, XL') })
  })

  it('resets a key by removing it', () => {
    expect(resetChange('p', 'XL')).toEqual({ node: 'p', estimates: { XL: null } })
  })
})
