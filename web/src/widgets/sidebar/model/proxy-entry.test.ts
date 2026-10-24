import type { ProxyReport } from '@/shared/api'
import { proxyEntry } from './proxy-entry'

const report = (ok: boolean): ProxyReport => ({
  ok,
  proxy_port: 27380,
  server_port: 27317,
  checks: [{ ok, what: 'alias 127.0.0.2 on lo0', hint: 'missing: run `ariane proxy setup`' }],
  routes: [
    { host: 'ariane.local', port: 27317, node: null, declared: true, listening: true },
    { host: 'dreamcatchr.local', port: 3000, node: 'slashome/dreamcatchr', declared: ok, listening: ok },
  ],
  problems: [],
})

describe('proxyEntry', () => {
  it('is green with the route count when every link holds', () => {
    expect(proxyEntry(report(true))).toEqual({ state: 'ok', count: 2, dot: 'bg-emerald-500', status: 'every link holds' })
  })

  it('is red when a link is broken', () => {
    const entry = proxyEntry(report(false))
    expect(entry.state).toBe('broken')
    expect(entry.dot).toBe('bg-red-500')
    expect(entry.count).toBe(2)
  })

  it('stays neutral, without a count, while loading or when the answer is not a report', () => {
    expect(proxyEntry(undefined)).toEqual({ state: 'unknown', count: undefined, dot: 'bg-muted-foreground/40', status: 'not checked yet' })
    expect(proxyEntry({} as ProxyReport).count).toBeUndefined()
  })
})
