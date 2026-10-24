import { routeOwner } from '@/entities/proxy'
import type { ProxyReport } from '@/shared/api'

const label = /^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/

export function isHost(name: string): boolean {
  return name.length > 0 && name.length <= 253 && name.split('.').every((part) => label.test(part))
}

export function suggestHost(node: string): string {
  const last = node.split('/').pop() ?? ''
  const name = last
    .toLowerCase()
    .replace(/[^a-z0-9-]+/g, '-')
    .replace(/^-+|-+$/g, '')
  return `${name === '' ? 'project' : name}.local`
}

export interface RouteDraft {
  node: string | null
  host: string
  port: string
}

export function parsePort(port: string): number | null {
  if (!/^\d{1,5}$/.test(port.trim())) {
    return null
  }
  const value = Number(port.trim())
  return value >= 1 && value <= 65535 ? value : null
}

export function routeProblem(draft: RouteDraft, report: Pick<ProxyReport, 'routes' | 'proxy_port' | 'server_port'>): string | null {
  if (draft.node === null) {
    return 'Choose the node that runs the server.'
  }
  const host = draft.host.trim()
  if (!isHost(host)) {
    return 'A host name: lowercase letters, digits and inner hyphens, separated by dots, such as app.local.'
  }
  const taken = report.routes.find((route) => route.host === host && route.node !== draft.node)
  if (taken !== undefined) {
    return `${host} is already the name of ${taken.node === null ? 'the interface' : `node ${routeOwner(taken)}`}.`
  }
  const port = parsePort(draft.port)
  if (port === null) {
    return 'A port between 1 and 65535.'
  }
  if (port === report.proxy_port) {
    return `${port} is the front door's port (proxy.port).`
  }
  if (port === report.server_port) {
    return `${port} is the interface's port (server.port).`
  }
  return null
}

export function needsSetup(host: string, report: Pick<ProxyReport, 'routes'>): boolean {
  return !report.routes.some((route) => route.host === host && route.declared)
}
