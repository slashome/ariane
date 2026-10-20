import type { NodeSummary } from '@/shared/api'

export function sortNodes(nodes: NodeSummary[]): NodeSummary[] {
  return [...nodes].sort((left, right) => left.path.localeCompare(right.path))
}
