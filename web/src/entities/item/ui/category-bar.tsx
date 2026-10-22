import type { Category } from '@/shared/api'
import { cn } from '@/shared/lib'
import { categoryOrder } from '../model/categories'
import { categoryFill } from '../model/category-tone'

// One stacked bar of items by status category, segments separated by a 2px gap, with a legend that
// names each category so identity never rests on color alone.
export function CategoryBar({
  counts,
  labels,
  legend = true,
  className,
}: {
  counts: Record<Category, number>
  labels: (category: Category) => string
  legend?: boolean
  className?: string
}) {
  const present = categoryOrder.filter((category) => counts[category] > 0)
  const total = present.reduce((sum, category) => sum + counts[category], 0)
  const summary = present.map((category) => `${labels(category)} ${counts[category]}`).join(', ')
  return (
    <div className={cn('space-y-1.5', className)}>
      <div className="flex h-2 gap-0.5 overflow-hidden rounded-full bg-muted" role="img" aria-label={summary || 'No item'}>
        {present.map((category) => (
          <div
            key={category}
            className={cn('first:rounded-l-full last:rounded-r-full', categoryFill(category))}
            style={{ width: `${(counts[category] / Math.max(total, 1)) * 100}%` }}
            title={`${labels(category)}: ${counts[category]}`}
          />
        ))}
      </div>
      {legend && (
        <ul className="flex flex-wrap gap-x-3 gap-y-0.5 text-xs text-muted-foreground">
          {present.map((category) => (
            <li key={category} className="flex items-center gap-1">
              <span className={`size-2 rounded-full ${categoryFill(category)}`} aria-hidden />
              {labels(category)} <span className="font-medium text-foreground">{counts[category]}</span>
            </li>
          ))}
          {present.length === 0 && <li>No work item</li>}
        </ul>
      )}
    </div>
  )
}
