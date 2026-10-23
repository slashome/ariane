import type { NodeSummary } from '@/shared/api'
import { firstStatusOf, statusLabel, workflowOf } from './workflow'

const nodes: NodeSummary[] = [
  {
    path: 'game',
    items: 2,
    workflow: [
      { id: 'a-faire', label: 'À faire', category: 'todo' },
      { id: 'a-tester', label: 'À tester', category: 'in_progress' },
      { id: 'en-cours', label: 'En cours', category: 'in_progress' },
    ],
  },
]

describe('workflow lookup', () => {
  it('labels a status with its workflow label and falls back to the id', () => {
    expect(statusLabel(nodes, 'game', 'a-faire')).toBe('À faire')
    expect(statusLabel(nodes, 'game', 'unknown')).toBe('unknown')
    expect(statusLabel(nodes, 'elsewhere', 'todo')).toBe('todo')
    expect(statusLabel(nodes, 'game', null)).toBeNull()
  })

  it('finds the first status of a category, in workflow order', () => {
    expect(firstStatusOf(workflowOf(nodes, 'game'), 'in_progress')?.id).toBe('a-tester')
    expect(firstStatusOf(workflowOf(nodes, 'game'), 'done')).toBeNull()
  })
})
