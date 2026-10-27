import { enumValues } from '@/entities/schema'
import type { Item, NodeSummary, Schema } from '@/shared/api'
import { foldText } from '@/shared/lib'
import { filterKeys, isRankedKey, isValues, rankedKeys, type Clause, type FilterKey, type ItemQuery, type RankedKey, type Span } from './query'

export interface Term {
  value: string
  label?: string
}

// The values the field knows: statuses of every workflow, categories and ordered values of the schema,
// handles of every directory, epics by id and by reference.
export interface QueryVocabulary {
  status: Term[]
  category: Term[]
  size: Term[]
  impact: Term[]
  priority: Term[]
  people: Term[]
  epic: Term[]
}

function unique(terms: Term[]): Term[] {
  const seen = new Map<string, Term>()
  for (const term of terms) {
    if (!seen.has(term.value)) {
      seen.set(term.value, term)
    }
  }
  return [...seen.values()]
}

export function buildVocabulary(nodes: NodeSummary[], schema: Schema | undefined, items: Item[] = []): QueryVocabulary {
  const epics = items.filter((item) => item.kind === 'epic' && item.id !== null && item.reference !== null)
  const ranked = (key: RankedKey) => enumValues(schema, key).map((value) => ({ value }))
  return {
    status: unique([
      ...nodes.flatMap((node) => node.workflow.map((status) => ({ value: status.id, label: status.label }))),
      ...(schema?.reflexion_statuses ?? []).map((value) => ({ value, label: 'reflexion' })),
    ]),
    category: (schema?.categories ?? []).map(({ id, label }) => ({ value: id, label })),
    size: ranked('size'),
    impact: ranked('impact'),
    priority: ranked('priority'),
    people: unique(
      nodes.flatMap((node) => Object.entries(node.people ?? {}).map(([handle, person]) => ({ value: handle, label: person.name }))),
    ).sort((left, right) => left.value.localeCompare(right.value)),
    epic: unique([
      ...epics.map((epic) => ({ value: epic.id as string, label: epic.title ?? undefined })),
      ...epics.map((epic) => ({ value: epic.reference as string, label: epic.title ?? undefined })),
    ]),
  }
}

// The ordered values of the ranked keys, which `size:<=M` compares along.
export type RankOrders = Partial<Record<RankedKey, string[]>>

export function rankOrders(vocabulary: QueryVocabulary): RankOrders {
  return Object.fromEntries(rankedKeys.map((key) => [key, vocabulary[key].map((term) => term.value)]))
}

export function valuesOf(vocabulary: QueryVocabulary, key: FilterKey): Term[] {
  return key === 'is' ? isValues.map((value) => ({ value })) : vocabulary[key]
}

export interface Issue extends Span {
  severity: 'error' | 'warning'
  message: string
}

function known(terms: Term[], value: string): boolean {
  const folded = foldText(value)
  return terms.some((term) => foldText(term.value) === folded)
}

function listed(terms: Term[]): string {
  return terms.map((term) => term.value).join(', ')
}

function issuesOf(clause: Clause, vocabulary: QueryVocabulary): Omit<Issue, 'start' | 'end'>[] {
  switch (clause.type) {
    case 'text':
      return []
    case 'unknown':
      return [
        {
          severity: 'warning',
          message: `“${clause.key}:” is not a filter, searched as text — filters: ${filterKeys.map((key) => `${key}:`).join(' ')} @person`,
        },
      ]
    case 'person':
      if (clause.handle === '') {
        return [{ severity: 'warning', message: '“@” needs a handle' }]
      }
      return known(vocabulary.people, clause.handle)
        ? []
        : [{ severity: 'error', message: `nobody is @${clause.handle} in the directories — known: ${listed(vocabulary.people) || 'nobody'}` }]
    case 'filter': {
      if (clause.comparator !== '=' && !isRankedKey(clause.key)) {
        return [{ severity: 'error', message: `“${clause.key}:” cannot compare; only ${rankedKeys.join(', ')} can` }]
      }
      if (clause.values.length === 0) {
        return [{ severity: 'warning', message: `“${clause.key}:” needs a value` }]
      }
      const terms = valuesOf(vocabulary, clause.key)
      return clause.values
        .filter((value) => !known(terms, value))
        .map((value) => ({ severity: 'error' as const, message: `no ${clause.key} “${value}” — known: ${listed(terms) || 'none'}` }))
    }
  }
}

// What the field reports about a query: keys it does not know, values no workflow, schema or directory
// has, filters left without a value. Never silently dropped.
export function checkQuery(query: ItemQuery, vocabulary: QueryVocabulary): Issue[] {
  return query.clauses.flatMap((clause) =>
    issuesOf(clause, vocabulary).map((issue) => ({ ...issue, start: clause.start, end: clause.end })),
  )
}
