import type { Category } from '@/shared/api'
import { cn } from '@/shared/lib'
import { categoryOrder } from '../model/categories'
import { categoryFill } from '../model/category-tone'
import './item.css'

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
    <div className={cn('category-bar', className)}>
      <div className="category-bar__track" role="img" aria-label={summary || 'No item'}>
        {present.map((category) => (
          <div
            key={category}
            className={cn('category-bar__segment', categoryFill(category))}
            style={{ width: `${(counts[category] / Math.max(total, 1)) * 100}%` }}
            title={`${labels(category)}: ${counts[category]}`}
          />
        ))}
      </div>
      {legend && (
        <ul className="category-bar__legend">
          {present.map((category) => (
            <li key={category} className="category-bar__legend-item">
              <span className={cn('category-bar__swatch', categoryFill(category))} aria-hidden />
              {labels(category)} <span className="category-bar__count">{counts[category]}</span>
            </li>
          ))}
          {present.length === 0 && <li>No work item</li>}
        </ul>
      )}
    </div>
  )
}
