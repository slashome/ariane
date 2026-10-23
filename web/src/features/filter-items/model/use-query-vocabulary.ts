import { useMemo } from 'react'
import { useNodes } from '@/entities/node'
import { useSchema } from '@/entities/schema'
import { buildVocabulary, type QueryVocabulary } from './vocabulary'

// The vocabulary of the search field, `null` until the nodes and the schema are loaded, so nothing is
// reported unknown before it can be known.
export function useQueryVocabulary(): QueryVocabulary | null {
  const { data: nodes } = useNodes()
  const { data: schema } = useSchema()
  return useMemo(() => (nodes && schema ? buildVocabulary(nodes, schema) : null), [nodes, schema])
}
