import { categoryOptions, useSchema } from '@/entities/schema'
import type { Category } from '@/shared/api'
import { MultiSelect } from '@/shared/ui'
import { useItemFilters } from '../model/use-item-filters'

export function ItemFiltersBar() {
  const { data: schema } = useSchema()
  const { filters, setCategories } = useItemFilters()
  return (
    <div className="flex flex-wrap items-center gap-2">
      <MultiSelect
        label="Status"
        options={categoryOptions(schema)}
        selected={filters.categories}
        onChange={(categories) => setCategories(categories as Category[])}
      />
    </div>
  )
}
