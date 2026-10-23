import { useMemo, useState, type KeyboardEvent, type RefObject } from 'react'
import { parseQuery } from './query'
import { applySuggestion, suggest, type Suggestion } from './suggest'
import { useItemFilters } from './use-item-filters'
import { useQueryVocabulary } from './use-query-vocabulary'
import { checkQuery } from './vocabulary'

// The state of the search field: the `?q=` it edits, what it reports, and the suggestions it shows for
// the token under the cursor, chosen with the arrows and Enter or Tab. `input` is the field, whose
// cursor the suggestions follow.
export function useQueryField(input: RefObject<HTMLInputElement | null>) {
  const { filters, setQuery } = useItemFilters()
  const vocabulary = useQueryVocabulary()
  const [cursor, setCursor] = useState(0)
  const [open, setOpen] = useState(false)
  const [active, setActive] = useState(0)
  const text = filters.query

  const issues = useMemo(() => (vocabulary === null ? [] : checkQuery(parseQuery(text), vocabulary)), [text, vocabulary])
  const suggestions = useMemo(() => (vocabulary === null ? null : suggest(text, cursor, vocabulary)), [text, cursor, vocabulary])
  const options = open ? (suggestions?.options ?? []) : []
  const highlighted = Math.min(active, Math.max(options.length - 1, 0))

  const follow = () => {
    setCursor(input.current?.selectionStart ?? text.length)
  }

  const choose = (option: Suggestion) => {
    if (suggestions === null) {
      return
    }
    const next = applySuggestion(text, suggestions, option)
    setQuery(next.text)
    setCursor(next.cursor)
    setActive(0)
    requestAnimationFrame(() => input.current?.setSelectionRange(next.cursor, next.cursor))
  }

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'Escape') {
      if (open && options.length > 0) {
        event.preventDefault()
        setOpen(false)
      }
      return
    }
    if (options.length === 0) {
      if (event.key === 'ArrowDown') {
        setOpen(true)
      }
      return
    }
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault()
      const step = event.key === 'ArrowDown' ? 1 : -1
      setActive((highlighted + step + options.length) % options.length)
    } else if (event.key === 'Enter' || (event.key === 'Tab' && !event.shiftKey)) {
      const option = options[highlighted]
      if (option !== undefined) {
        event.preventDefault()
        choose(option)
      }
    }
  }

  return {
    open,
    text,
    issues,
    options,
    active: highlighted,
    choose,
    onKeyDown,
    onChange: (value: string, position: number | null) => {
      setQuery(value)
      setCursor(position ?? value.length)
      setActive(0)
      setOpen(true)
    },
    onSelect: follow,
    onFocus: () => {
      follow()
      setOpen(true)
    },
    onBlur: () => setOpen(false),
  }
}
