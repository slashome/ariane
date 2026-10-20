import { Search } from 'lucide-react'
import { Input } from '@/shared/ui'
import { useItemFilters } from '../model/use-item-filters'

export function ItemSearchInput() {
  const { filters, setQuery } = useItemFilters()
  return (
    <div className="relative w-72">
      <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" aria-hidden />
      <Input
        type="search"
        aria-label="Search items"
        placeholder="Search title, id, node, path…"
        className="pl-8"
        value={filters.query}
        onChange={(event) => setQuery(event.target.value)}
      />
    </div>
  )
}
