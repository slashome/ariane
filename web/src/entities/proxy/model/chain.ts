import type { ProxyReport, ProxyRoute } from '@/shared/api'
import { displayNode } from '@/shared/lib'

export type ChainState = 'ok' | 'broken' | 'unknown'

export function chainState(report: ProxyReport | undefined): ChainState {
  if (report === undefined || typeof report.ok !== 'boolean') {
    return 'unknown'
  }
  return report.ok ? 'ok' : 'broken'
}

export function brokenChecks(report: ProxyReport): number {
  return report.checks.filter((check) => !check.ok).length
}

export function routeOwner(route: Pick<ProxyRoute, 'node'>): string {
  return route.node === null ? 'interface' : displayNode(route.node)
}
