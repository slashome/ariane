import type { Item } from './types'

// An item with every field set, for tests: a live task of node `p` unless `overrides` say otherwise.
export function makeItem(overrides: Partial<Item> = {}): Item {
  const id = overrides.id ?? 'T001'
  const node = overrides.node ?? 'p'
  return {
    node,
    id,
    reference: id === null ? null : `${node}#${id}`,
    title: id,
    kind: 'task',
    archived: false,
    status: 'todo',
    category: 'todo',
    size: null,
    impact: null,
    priority: null,
    created: null,
    type: null,
    release: null,
    milestone: null,
    parent: null,
    depends_on: [],
    external: [],
    branches: [],
    pull_requests: [],
    version: 1,
    path: `tree/${node}/tasks/${id}.md`,
    ...overrides,
  }
}
