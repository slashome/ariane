import type { ChainState } from '@/entities/proxy'
import { chainState } from '@/entities/proxy'
import type { ProxyReport } from '@/shared/api'

export interface ProxyEntry {
  state: ChainState
  count: number | undefined
  dot: string
  status: string
}

const dots: Record<ChainState, string> = {
  ok: 'bg-emerald-500',
  broken: 'bg-red-500',
  unknown: 'bg-muted-foreground/40',
}

const statuses: Record<ChainState, string> = {
  ok: 'every link holds',
  broken: 'a link is broken',
  unknown: 'not checked yet',
}

export function proxyEntry(report: ProxyReport | undefined): ProxyEntry {
  const state = chainState(report)
  return {
    state,
    count: Array.isArray(report?.routes) ? report.routes.length : undefined,
    dot: dots[state],
    status: statuses[state],
  }
}
