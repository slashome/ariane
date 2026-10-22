import { isClosed } from '@/entities/item'
import type { Finding, Item } from '@/shared/api'

export interface NavCounts {
  tasks: number
  open: number
  toEstimate: number
  backlog: number
  stories: number
  reflexions: number
  settled: number
  ressources: number
  errors: number
}

// The number shown beside each menu entry, from the items already scoped to the node and the archive
// toggle, and the doctor findings of that node.
export function navCounts(items: Item[], findings: Finding[]): NavCounts {
  const tasks = items.filter((item) => item.kind === 'task')
  const open = tasks.filter((item) => !isClosed(item.category))
  const reflexions = items.filter((item) => item.kind === 'reflexion')
  return {
    tasks: tasks.length,
    open: open.length,
    toEstimate: open.filter((item) => item.size === null || item.impact === null).length,
    backlog: tasks.filter((item) => item.category === 'backlog').length,
    stories: items.filter((item) => item.kind === 'story').length,
    reflexions: reflexions.filter((item) => item.status !== 'settled').length,
    settled: reflexions.filter((item) => item.status === 'settled').length,
    ressources: items.filter((item) => item.kind === 'ressource').length,
    errors: findings.filter((finding) => finding.severity === 'error').length,
  }
}
