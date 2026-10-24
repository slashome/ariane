import type { ProxyReport } from '@/shared/api'
import { isHost, needsSetup, parsePort, routeProblem, suggestHost } from './route-form'

const report: Pick<ProxyReport, 'routes' | 'proxy_port' | 'server_port'> = {
  proxy_port: 27380,
  server_port: 27317,
  routes: [
    { host: 'ariane.local', port: 27317, node: null, declared: true, listening: true },
    { host: 'dreamcatchr.local', port: 3000, node: 'slashome/dreamcatchr', declared: false, listening: false },
  ],
}

describe('a route draft', () => {
  it('is valid with a free lowercase host and a free port', () => {
    expect(routeProblem({ node: 'cmc/lib', host: 'lib.local', port: '6006' }, report)).toBeNull()
    expect(routeProblem({ node: 'slashome/dreamcatchr', host: 'dreamcatchr.local', port: '3001' }, report)).toBeNull()
  })

  it('names what is wrong, in the order the server checks it', () => {
    expect(routeProblem({ node: null, host: 'a.local', port: '1' }, report)).toMatch(/Choose the node/)
    expect(routeProblem({ node: 'p', host: 'App.local', port: '1' }, report)).toMatch(/host name/)
    expect(routeProblem({ node: 'p', host: 'ariane.local', port: '1' }, report)).toBe('ariane.local is already the name of the interface.')
    expect(routeProblem({ node: 'p', host: 'dreamcatchr.local', port: '1' }, report)).toBe('dreamcatchr.local is already the name of node slashome/dreamcatchr.')
    expect(routeProblem({ node: 'p', host: 'p.local', port: '70000' }, report)).toMatch(/between 1 and 65535/)
    expect(routeProblem({ node: 'p', host: 'p.local', port: '27380' }, report)).toMatch(/front door/)
    expect(routeProblem({ node: 'p', host: 'p.local', port: '27317' }, report)).toMatch(/interface's port/)
  })

  it('reads hosts and ports as the server does', () => {
    expect(isHost('dream-catchr.local')).toBe(true)
    expect(isHost('localhost')).toBe(true)
    expect(isHost('-a.local')).toBe(false)
    expect(isHost('a..local')).toBe(false)
    expect(isHost('a.local:80')).toBe(false)
    expect(parsePort('0')).toBeNull()
    expect(parsePort(' 3000 ')).toBe(3000)
    expect(parsePort('3e3')).toBeNull()
  })

  it('suggests a host from the node and tells when /etc/hosts must be rewritten', () => {
    expect(suggestHost('slashome/Dream Catchr')).toBe('dream-catchr.local')
    expect(suggestHost('')).toBe('project.local')
    expect(needsSetup('ariane.local', report)).toBe(false)
    expect(needsSetup('dreamcatchr.local', report)).toBe(true)
    expect(needsSetup('new.local', report)).toBe(true)
  })
})
