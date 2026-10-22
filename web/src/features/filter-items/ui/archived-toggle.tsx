import { Archive } from 'lucide-react'
import { Switch } from '@/shared/ui'
import { useItemFilters } from '../model/use-item-filters'

export function ArchivedToggle() {
  const { filters, setIncludeArchived } = useItemFilters()
  return (
    <label className="flex cursor-pointer items-center gap-1.5" title="Include the items under archives/ in every view">
      <Archive className="size-3.5" aria-hidden />
      Archived
      <Switch
        checked={filters.includeArchived}
        onCheckedChange={setIncludeArchived}
        aria-label="Include archived items"
        className="scale-75"
      />
    </label>
  )
}
