// The search field's language, GitHub style: `status:` `category:` `is:open|closed` `size:<=M` `impact:`
// `priority:` `@handle`, `-` to negate, quotes for a phrase, and free text for the rest.

export const filterKeys = ['status', 'category', 'is', 'size', 'impact', 'priority'] as const
export type FilterKey = (typeof filterKeys)[number]

// The keys whose values are ordered by the schema, which accept `<`, `<=`, `>`, `>=`.
export const rankedKeys = ['size', 'impact', 'priority'] as const
export type RankedKey = (typeof rankedKeys)[number]

export const isValues = ['open', 'closed'] as const

export type Comparator = '=' | '<' | '<=' | '>' | '>='

// Where a clause stands in the typed text, in UTF-16 units as the input's selection counts them.
export interface Span {
  start: number
  end: number
}

export type Clause = Span & { negated: boolean } & (
    | { type: 'text'; text: string }
    // A filter; `values` are alternatives, written `status:a,b`; none while it is being typed.
    | { type: 'filter'; key: FilterKey; comparator: Comparator; values: string[] }
    // Items waiting for this person; an empty handle while it is being typed.
    | { type: 'person'; handle: string }
    // `word:…` where `word` is no filter: searched as text, and reported.
    | { type: 'unknown'; key: string; text: string }
  )

export interface ItemQuery {
  clauses: Clause[]
}

export function isFilterKey(key: string): key is FilterKey {
  return (filterKeys as readonly string[]).includes(key)
}

export function isRankedKey(key: string): key is RankedKey {
  return (rankedKeys as readonly string[]).includes(key)
}

// The tokens of a text with their spans: runs of non-blank characters, a quoted phrase kept whole.
export function tokenize(text: string): Span[] {
  const spans: Span[] = []
  let index = 0
  while (index < text.length) {
    if (/\s/.test(text.charAt(index))) {
      index += 1
      continue
    }
    const start = index
    const quote = text[index] === '"' || (text[index] === '-' && text[index + 1] === '"') ? text.indexOf('"', index) : -1
    if (quote !== -1) {
      const closing = text.indexOf('"', quote + 1)
      index = closing === -1 ? text.length : closing + 1
    } else {
      while (index < text.length && !/\s/.test(text.charAt(index))) {
        index += 1
      }
    }
    spans.push({ start, end: index })
  }
  return spans
}

const comparatorPattern = /^(<=|>=|<|>|=)?(.*)$/s

export function parseClause(raw: string, span: Span): Clause {
  const negated = raw.length > 1 && raw.startsWith('-')
  const body = negated ? raw.slice(1) : raw
  if (body.startsWith('"')) {
    return { ...span, negated, type: 'text', text: body.slice(1, body.endsWith('"') && body.length > 1 ? -1 : undefined) }
  }
  if (body.startsWith('@')) {
    return { ...span, negated, type: 'person', handle: body.slice(1) }
  }
  const pair = /^([A-Za-z_]+):(.*)$/s.exec(body)
  if (pair === null) {
    return { ...span, negated, type: 'text', text: body }
  }
  const [, written = '', value = ''] = pair
  const key = written.toLowerCase()
  if (!isFilterKey(key)) {
    return { ...span, negated, type: 'unknown', key, text: body }
  }
  const [, comparator = '=', rest = ''] = comparatorPattern.exec(value) ?? []
  const values = rest.split(',').filter((value) => value !== '')
  return { ...span, negated, type: 'filter', key, comparator: comparator as Comparator, values }
}

export function parseQuery(text: string): ItemQuery {
  return { clauses: tokenize(text).map((span) => parseClause(text.slice(span.start, span.end), span)) }
}

export function formatClause(clause: Clause): string {
  const sign = clause.negated ? '-' : ''
  switch (clause.type) {
    case 'text':
      return `${sign}${/\s/.test(clause.text) || clause.text.startsWith('"') ? `"${clause.text}"` : clause.text}`
    case 'filter':
      return `${sign}${clause.key}:${clause.comparator === '=' ? '' : clause.comparator}${clause.values.join(',')}`
    case 'person':
      return `${sign}@${clause.handle}`
    case 'unknown':
      return `${sign}${clause.text}`
  }
}

// The text of a query, each clause written the way the field reads it, separated by one space.
export function formatQuery(query: ItemQuery): string {
  return query.clauses.map(formatClause).join(' ')
}

// The free words of a query, to highlight in the items it finds; negated ones are not in them.
export function highlightTerms(query: ItemQuery): string[] {
  return query.clauses.flatMap((clause) =>
    clause.negated ? [] : clause.type === 'text' || clause.type === 'unknown' ? [clause.text] : [],
  )
}
