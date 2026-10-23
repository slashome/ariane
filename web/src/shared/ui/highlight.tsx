import { createContext, useContext, useMemo, type ReactNode } from 'react'
import { highlightRanges, splitByRanges, type Range } from '@/shared/lib'

// The words to highlight wherever an item's text is shown: those of the search field, set once by the app.
const TermsContext = createContext<readonly string[]>([])

export function HighlightTerms({ terms, children }: { terms: readonly string[]; children: ReactNode }) {
  return <TermsContext.Provider value={terms}>{children}</TermsContext.Provider>
}

// The words of the search field, for a view that picks what to show around them.
export function useHighlightTerms(): readonly string[] {
  return useContext(TermsContext)
}

// A text with its matches marked: the given ranges (from `ariane search`), the given terms, or else the
// terms of the search field.
export function Highlight({ text, terms, ranges }: { text: string; terms?: readonly string[]; ranges?: readonly Range[] }) {
  const shared = useContext(TermsContext)
  const active = terms ?? shared
  const segments = useMemo(() => splitByRanges(text, ranges ?? highlightRanges(text, active)), [text, ranges, active])
  if (!segments.some((segment) => segment.match)) {
    return text
  }
  return segments.map((segment, index) =>
    segment.match ? (
      <mark key={index} className="rounded-xs bg-amber-200/80 text-inherit dark:bg-amber-400/30">
        {segment.text}
      </mark>
    ) : (
      segment.text
    ),
  )
}
