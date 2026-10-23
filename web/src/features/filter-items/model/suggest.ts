import { foldText } from '@/shared/lib'
import { filterKeys, isFilterKey, isRankedKey, tokenize, type FilterKey } from './query'
import { valuesOf, type QueryVocabulary, type Term } from './vocabulary'

export interface Suggestion {
  // The token that replaces the one under the cursor.
  value: string
  label: string
  description?: string
  // A whole clause, followed by a space once chosen; a key waits for its value.
  complete: boolean
}

export interface Suggestions {
  // The span of the token under the cursor, which a suggestion replaces.
  start: number
  end: number
  options: Suggestion[]
}

const keyHints: Record<FilterKey, string> = {
  status: 'a status of the workflows',
  category: 'a status category',
  is: 'open or closed',
  size: 'a size, or a comparison such as size:<=M',
  impact: 'an impact, or a comparison such as impact:>=medium',
  priority: 'a priority, or a comparison such as priority:>low',
}

const limit = 8

function startsWith(term: string, prefix: string): boolean {
  return foldText(term).startsWith(foldText(prefix))
}

// The terms whose value, or a word of whose label, starts with what is typed.
function matching(terms: Term[], prefix: string): Term[] {
  return terms.filter((term) => startsWith(term.value, prefix) || (term.label ?? '').split(/\s+/).some((word) => startsWith(word, prefix)))
}

// What the field proposes for the token under the cursor: the keys first, then, once a key is typed,
// its known values; `@` proposes the people. Free text proposes nothing.
export function suggest(text: string, cursor: number, vocabulary: QueryVocabulary): Suggestions {
  const span = tokenize(text).find((token) => token.start <= cursor && cursor <= token.end) ?? { start: cursor, end: cursor }
  const typed = text.slice(span.start, cursor)
  const sign = typed.startsWith('-') ? '-' : ''
  const body = typed.slice(sign.length)
  const found = (options: Suggestion[]) => ({ ...span, options: options.slice(0, limit) })

  if (body.startsWith('@')) {
    return found(
      matching(vocabulary.people, body.slice(1)).map((person) => ({
        value: `${sign}@${person.value}`,
        label: `@${person.value}`,
        description: person.label,
        complete: true,
      })),
    )
  }
  const pair = /^([A-Za-z_]+):(<=|>=|<|>|=)?(.*)$/.exec(body)
  if (pair === null) {
    if (body.startsWith('"')) {
      return found([])
    }
    const keys = filterKeys
      .filter((key) => key.startsWith(body.toLowerCase()))
      .map((key) => ({ value: `${sign}${key}:`, label: `${key}:`, description: keyHints[key], complete: false }))
    const person = '@'.startsWith(body) || body === '' ? [{ value: `${sign}@`, label: '@person', description: 'items waiting for someone', complete: false }] : []
    return found([...keys, ...person])
  }
  const [, typedKey = '', comparator = '', typedValues = ''] = pair
  const key = typedKey.toLowerCase()
  if (!isFilterKey(key)) {
    return found([])
  }
  const written = typedValues.split(',')
  const current = written.pop() ?? ''
  const before = written.map((value) => `${value},`).join('')
  const head = `${sign}${key}:${comparator}`
  const values = matching(valuesOf(vocabulary, key), current)
    .filter((term) => !written.some((value) => foldText(value) === foldText(term.value)))
    .map((term) => ({
    value: `${head}${before}${term.value}`,
    label: term.value,
    description: term.label,
    complete: true,
  }))
  const comparisons =
    isRankedKey(key) && comparator === '' && current === '' && before === ''
      ? ['<=', '>='].map((operator) => ({ value: `${head}${operator}`, label: `${key}:${operator}`, description: 'compare along the schema order', complete: false }))
      : []
  return found([...values, ...comparisons])
}

// The text once a suggestion replaces the token under the cursor, and where the cursor goes.
export function applySuggestion(text: string, suggestions: Suggestions, option: Suggestion): { text: string; cursor: number } {
  const after = text.slice(suggestions.end)
  const space = option.complete && !after.startsWith(' ') ? ' ' : ''
  const head = `${text.slice(0, suggestions.start)}${option.value}${space}`
  return { text: `${head}${after}`, cursor: head.length + (option.complete && space === '' ? 1 : 0) }
}
