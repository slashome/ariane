import type { NodeSummary, Schema } from '@/shared/api'
import { buildVocabulary } from './vocabulary'

// A schema, two nodes and the vocabulary they give, for the tests of the search field.
export const schema = {
  version: 1,
  categories: [
    { id: 'todo', label: 'To do' },
    { id: 'done', label: 'Done' },
  ],
  id_prefixes: { task: 'T', story: 'S', reflexion: 'R', ressource: 'D', unknown: 'T' },
  keys: [
    { name: 'size', type: 'enum', values: ['XS', 'S', 'M', 'L', 'XL'] },
    { name: 'impact', type: 'enum', values: ['low', 'medium', 'high'] },
    { name: 'priority', type: 'enum', values: ['low', 'medium', 'high'] },
  ],
  reflexion_statuses: ['open', 'settled'],
} satisfies Schema

export const nodes: NodeSummary[] = [
  { path: 'a', has_index: true, items: 1, workflow: [{ id: 'a-faire', label: 'À faire', category: 'todo' }], people: { keyvan: { name: 'Keyvan Coen' } } },
  { path: 'b', has_index: true, items: 1, workflow: [{ id: 'a-faire', label: 'Todo', category: 'todo' }, { id: 'fait', label: 'Fait', category: 'done' }], people: { andrea: { name: 'Andrea Martel' } } },
]

export const vocabulary = buildVocabulary(nodes, schema)
