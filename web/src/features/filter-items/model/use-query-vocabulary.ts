import { useMemo } from 'react'
import { useItems } from '@/entities/item'
import { useNodes } from '@/entities/node'
import { useSchema } from '@/entities/schema'
import { buildVocabulary, type QueryVocabulary } from './vocabulary'

// The vocabulary of the search field, `null` until the nodes, the schema and the items are loaded, so
// nothing is reported unknown before it can be known.
export function useQueryVocabulary(): QueryVocabulary | null {
  const { data: nodes } = useNodes()
  const { data: schema } = useSchema()
  const { data: items } = useItems()
  return useMemo(() => (nodes && schema && items ? buildVocabulary(nodes, schema, items) : null), [nodes, schema, items])
}
