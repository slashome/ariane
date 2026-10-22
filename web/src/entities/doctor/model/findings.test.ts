import type { Finding } from '@/shared/api'
import { checkLabel, countBySeverity, scopeFindings } from './findings'

const finding = (node: string | null, severity: Finding['severity'] = 'error'): Finding => ({
  check: 'file_name',
  severity,
  node,
  path: null,
  message: 'm',
  remedy: { kind: 'command', command: 'ariane fix' },
})

describe('doctor findings', () => {
  const findings = [finding('cmc/cmc-lib'), finding('cmc', 'warning'), finding('slashome'), finding(null)]

  it('keep those of the node in scope, and those about the whole tree', () => {
    expect(scopeFindings(findings, 'cmc').map((entry) => entry.node)).toEqual(['cmc/cmc-lib', 'cmc', null])
    expect(scopeFindings(findings, null)).toHaveLength(4)
  })

  it('count errors and warnings and name their check', () => {
    expect(countBySeverity(findings)).toEqual({ errors: 3, warnings: 1 })
    expect(checkLabel('dangling_reference')).toBe('Dangling reference')
  })
})
