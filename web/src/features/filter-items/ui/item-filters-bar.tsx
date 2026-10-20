import { categoryOptions, kindOptions, useSchema } from '@/entities/schema'
import type { Category, Kind } from '@/shared/api'
import { MultiSelect, Switch } from '@/shared/ui'
import { useItemFilters } from '../model/use-item-filters'

export function ItemFiltersBar() {
  const { data: schema } = useSchema()
  const { filters, setKinds, setCategories, setIncludeArchived } = useItemFilters()
  return (
    <div className="flex flex-wrap items-center gap-2">
      <MultiSelect
        label="Kind"
        options={kindOptions(schema)}
        selected={filters.kinds}
        onChange={(kinds) => setKinds(kinds as Kind[])}
      />
      <MultiSelect
        label="Status"
        options={categoryOptions(schema)}
        selected={filters.categories}
        onChange={(categories) => setCategories(categories as Category[])}
      />
      <label className="ml-1 flex items-center gap-2 text-sm text-muted-foreground">
        <Switch checked={filters.includeArchived} onCheckedChange={setIncludeArchived} />
        Include archives
      </label>
    </div>
  )
}
