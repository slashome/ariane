import { useMemo, type ReactNode } from 'react'
import { HighlightTerms } from '@/shared/ui'
import { highlightTerms, parseQuery } from '../model/query'
import { useItemFilters } from '../model/use-item-filters'

// Highlights the free words of the search field in every item shown below it.
export function QueryHighlight({ children }: { children: ReactNode }) {
  const { filters } = useItemFilters()
  const terms = useMemo(() => highlightTerms(parseQuery(filters.query)), [filters.query])
  return <HighlightTerms terms={terms}>{children}</HighlightTerms>
}
