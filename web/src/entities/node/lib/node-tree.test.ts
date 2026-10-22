import type { NodeSummary } from '@/shared/api'
import { hasDescendants, nodeTree } from './node-tree'

const node = (path: string): NodeSummary => ({ path, has_index: true, items: 0, workflow: [] })

describe('nodeTree', () => {
  const nodes = ['', 'slashome', 'cmc/cmc-lib', 'cmc', 'cmc/cmc-frontend', 'cmc/cmc-frontend/workspace', 'slashome/ariane', 'atlas/loop'].map(node)

  it('draws the nodes as a tree, children under their nearest parent node', () => {
    expect(nodeTree(nodes).map((row) => `${row.prefix}${row.name}`)).toEqual([
      'atlas/loop',
      'cmc',
      '├─ cmc-frontend',
      '│  └─ workspace',
      '└─ cmc-lib',
      'slashome',
      '└─ ariane',
    ])
  })

  it('tells which nodes have sub-nodes', () => {
    const rows = nodeTree(nodes)
    expect(rows.filter((row) => row.hasChildren).map((row) => row.node.path)).toEqual(['cmc', 'cmc/cmc-frontend', 'slashome'])
    expect(hasDescendants(nodes, 'cmc')).toBe(true)
    expect(hasDescendants(nodes, 'cmc/cmc-lib')).toBe(false)
    expect(hasDescendants(nodes, '')).toBe(true)
    expect(hasDescendants(nodes, null)).toBe(false)
  })
})
