import type { Finding } from '@/shared/api'
import { isWithinNode } from '@/shared/lib'

// The findings of the node in scope, with those of its sub-nodes when asked; those about the whole tree
// (an unreadable path, an invalid node.toml) show wherever they apply, which `doctor` reports without a node.
export function scopeFindings(findings: Finding[], node: string | null, includeDescendants = true): Finding[] {
  if (node === null) {
    return findings
  }
  return findings.filter(
    (finding) => finding.node === null || (includeDescendants ? isWithinNode(finding.node, node) : finding.node === node),
  )
}

export function countBySeverity(findings: Finding[]): { errors: number; warnings: number } {
  const errors = findings.filter((finding) => finding.severity === 'error').length
  return { errors, warnings: findings.length - errors }
}

// `dangling_reference` reads "Dangling reference".
export function checkLabel(check: string): string {
  const words = check.replace(/_/g, ' ')
  return words.charAt(0).toUpperCase() + words.slice(1)
}
