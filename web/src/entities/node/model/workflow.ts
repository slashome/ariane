import type { Category, NodeSummary, WorkflowStatus } from '@/shared/api'

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
