export const queryKeys = {
  items: ['items'],
  nodes: ['nodes'],
  schema: ['schema'],
  repo: ['repo'],
  diff: (path: string) => ['diff', path] as const,
  diffs: ['diff'],
  dashboard: ['dashboard'],
} as const
