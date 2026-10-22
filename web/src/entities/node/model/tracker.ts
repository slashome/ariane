import type { NodeSummary, Tracker } from '@/shared/api'
import { findNode } from './workflow'

export function trackerOf(nodes: NodeSummary[], path: string): Tracker | null {
  return findNode(nodes, path)?.tracker ?? null
}

// An `external` entry reads `<connector>:<id>`; it links only to the tracker the node is bound to.
export function externalUrl(tracker: Tracker | null, entry: string): string | null {
  const separator = entry.indexOf(':')
  if (tracker === null || separator <= 0) {
    return null
  }
  const connector = entry.slice(0, separator)
  const id = entry.slice(separator + 1)
  if (connector !== tracker.kind || !/^\d+$/.test(id)) {
    return null
  }
  if (connector === 'monday') {
    const { account, board } = tracker
    if (typeof account !== 'string' || !/^[a-z0-9-]+$/i.test(account) || !/^\d+$/.test(String(board ?? ''))) {
      return null
    }
    return `https://${account}.monday.com/boards/${String(board)}/pulses/${id}`
  }
  return null
}
