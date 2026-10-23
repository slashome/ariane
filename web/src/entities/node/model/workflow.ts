import type { Category, NodeSummary, Person, WorkflowStatus } from '@/shared/api'

export function findNode(nodes: NodeSummary[], path: string): NodeSummary | undefined {
  return nodes.find((node) => node.path === path)
}

export function workflowOf(nodes: NodeSummary[], path: string): WorkflowStatus[] {
  return findNode(nodes, path)?.workflow ?? []
}

export function statusLabel(nodes: NodeSummary[], path: string, status: string | null): string | null {
  if (status === null) {
    return null
  }
  return workflowOf(nodes, path).find((entry) => entry.id === status)?.label ?? status
}

export function firstStatusOf(workflow: WorkflowStatus[], category: Category): WorkflowStatus | null {
  return workflow.find((entry) => entry.category === category) ?? null
}

// The people an item of `path` may wait for: its node's `[people]`, inherited ones included.
export function peopleOf(nodes: NodeSummary[], path: string): Record<string, Person> {
  return findNode(nodes, path)?.people ?? {}
}

export interface PersonaLabel {
  id: string
  name: string
}

export function personasOf(nodes: NodeSummary[], path: string, ids: string[]): PersonaLabel[] {
  const declared = findNode(nodes, path)?.personas ?? {}
  return ids.map((id) => ({ id, name: declared[id]?.name ?? id }))
}
