import type { EstimateKey, EstimatesChange, Pricing } from '@/shared/api'
import { displayNode } from '@/shared/lib'

export const sizeKeys = ['XS', 'S', 'M', 'L', 'XL'] as const

export const estimateKeys: EstimateKey[] = [...sizeKeys, 'default', 'test_ratio', 'tester']

export const testers = ['owner', 'others'] as const

export type OriginKind = 'own' | 'inherited' | 'default'

export interface Origin {
  kind: OriginKind
  label: string
}

export const estimateLabels: Record<EstimateKey, string> = {
  XS: 'XS',
  S: 'S',
  M: 'M',
  L: 'L',
  XL: 'XL',
  default: 'Default size',
  test_ratio: 'Test ratio',
  tester: 'Tester',
}

export function originOf(node: string, from: string | null | undefined): Origin {
  if (from === null || from === undefined) {
    return { kind: 'default', label: 'default' }
  }
  if (from === node) {
    return { kind: 'own', label: 'own' }
  }
  return { kind: 'inherited', label: `from ${displayNode(from)}` }
}

export function nodeTomlPath(node: string): string {
  return node === '' ? 'tree/node.toml' : `tree/${node}/node.toml`
}

export type EstimateEdit = { change: EstimatesChange } | { error: string }

export function estimateEdit(node: string, key: EstimateKey, raw: string, current: Pricing): EstimateEdit | null {
  const text = raw.trim()
  if (text === String(current[key])) {
    return null
  }
  if (key === 'default') {
    return (sizeKeys as readonly string[]).includes(text)
      ? { change: { node, estimates: { default: text } } }
      : { error: `The default size is one of ${sizeKeys.join(', ')}` }
  }
  if (key === 'tester') {
    return (testers as readonly string[]).includes(text)
      ? { change: { node, estimates: { tester: text } } }
      : { error: 'The tester is owner or others' }
  }
  const value = Number(text)
  if (text === '' || !Number.isFinite(value)) {
    return { error: `${estimateLabels[key]} is a number` }
  }
  if (key === 'test_ratio') {
    return value >= 0 ? { change: { node, estimates: { test_ratio: value } } } : { error: 'The test ratio is zero or above' }
  }
  return value > 0 ? { change: { node, estimates: { [key]: value } } } : { error: `${key} is a number of hours above zero` }
}

export function resetChange(node: string, key: EstimateKey): EstimatesChange {
  return { node, estimates: { [key]: null } }
}
