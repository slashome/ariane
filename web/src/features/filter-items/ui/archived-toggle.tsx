import { Icon, Switch } from '@/shared/ui'
import { useItemFilters } from '../model/use-item-filters'
import './filter-items.css'

export function ArchivedToggle() {
  const { filters, setIncludeArchived } = useItemFilters()
  return (
    <label className="archived-toggle" title="Include the items under archives/ in every view">
      <Icon name="archive" className="archived-toggle__icon" />
      Archived
      <Switch
        checked={filters.includeArchived}
        onCheckedChange={setIncludeArchived}
        aria-label="Include archived items"
        className="archived-toggle__switch"
      />
    </label>
  )
}
