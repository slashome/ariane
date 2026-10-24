import type { ProxyReport } from '@/shared/api'
import { brokenChecks, chainState, routeOwner } from './chain'

const report = (ok: boolean): ProxyReport => ({
  ok,
  proxy_port: 27380,
  server_port: 27317,
  checks: [
    { ok: true, what: 'alias 127.0.0.2 on lo0', hint: '' },
    { ok, what: 'front door on 127.0.0.1:27380', hint: 'is `ariane serve` or `ariane proxy` running?' },
  ],
  routes: [],
  problems: [],
})

describe('chainState', () => {
  it('holds when every link holds, breaks when one does not', () => {
    expect(chainState(report(true))).toBe('ok')
    expect(chainState(report(false))).toBe('broken')
  })

  it('is unknown while nothing came back, or something that is not a report', () => {
    expect(chainState(undefined)).toBe('unknown')
    expect(chainState({} as ProxyReport)).toBe('unknown')
  })
})

describe('the report', () => {
  it('counts its broken links and names who owns each route', () => {
    expect(brokenChecks(report(false))).toBe(1)
    expect(brokenChecks(report(true))).toBe(0)
    expect(routeOwner({ node: null })).toBe('interface')
    expect(routeOwner({ node: 'slashome/dreamcatchr' })).toBe('slashome/dreamcatchr')
    expect(routeOwner({ node: '' })).toBe('(home)')
  })
})
