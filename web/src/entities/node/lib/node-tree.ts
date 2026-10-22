import type { NodeSummary } from '@/shared/api'
import { isWithinNode } from '@/shared/lib'

export interface TreeRow {
  node: NodeSummary
  // `├─ `, `└─ ` and `│  ` drawn before the node's name, as `tree` does in a terminal.
  prefix: string
  name: string
  depth: number
  hasChildren: boolean
}

// The nearest node of `nodes` that `path` sits in, the home node aside; `null` for a top-level node.
function parentOf(path: string, paths: Set<string>): string | null {
  const segments = path.split('/')
  for (let length = segments.length - 1; length > 0; length -= 1) {
    const candidate = segments.slice(0, length).join('/')
    if (paths.has(candidate)) {
      return candidate
    }
  }
  return null
}

// The nodes as a tree, depth first in path order. The home node, which every node descends from, is
// left out: it stands for the whole tree and is listed apart. A child shows the part of its path below
// its parent.
export function nodeTree(nodes: NodeSummary[]): TreeRow[] {
  const listed = nodes.filter((node) => node.path !== '').sort((left, right) => left.path.localeCompare(right.path))
  const paths = new Set(listed.map((node) => node.path))
  const children = new Map<string | null, NodeSummary[]>()
  for (const node of listed) {
    const parent = parentOf(node.path, paths)
    children.set(parent, [...(children.get(parent) ?? []), node])
  }
  const rows: TreeRow[] = []
  const walk = (parent: string | null, indent: string, depth: number) => {
    const siblings = children.get(parent) ?? []
    siblings.forEach((node, index) => {
      const last = index === siblings.length - 1
      rows.push({
        node,
        prefix: depth === 0 ? '' : `${indent}${last ? '└─ ' : '├─ '}`,
        name: parent === null ? node.path : node.path.slice(parent.length + 1),
        depth,
        hasChildren: children.has(node.path),
      })
      walk(node.path, depth === 0 ? '' : `${indent}${last ? '   ' : '│  '}`, depth + 1)
    })
  }
  walk(null, '', 0)
  return rows
}

export function hasDescendants(nodes: NodeSummary[], path: string | null): boolean {
  return path !== null && nodes.some((node) => node.path !== path && isWithinNode(node.path, path))
}
