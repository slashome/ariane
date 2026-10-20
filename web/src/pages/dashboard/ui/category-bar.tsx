import { categoryOrder } from '@/entities/item'
import type { Category } from '@/shared/api'

const fills: Record<Category, string> = {
  backlog: 'bg-slate-400',
  todo: 'bg-blue-500',
  in_progress: 'bg-amber-500',
  blocked: 'bg-red-500',
  done: 'bg-green-500',
  dropped: 'bg-muted-foreground/40',
}

export function CategoryBar({
  counts,
  total,
  labels,
}: {
  counts: Record<Category, number>
  total: number
  labels: (category: Category) => string
}) {
  const present = categoryOrder.filter((category) => counts[category] > 0)
  return (
    <div className="space-y-1.5">
      <div className="flex h-2 overflow-hidden rounded-full bg-muted" role="img" aria-label="Items by status category">
        {present.map((category) => (
          <div
            key={category}
            className={fills[category]}
            style={{ width: `${(counts[category] / Math.max(total, 1)) * 100}%` }}
          />
        ))}
      </div>
      <ul className="flex flex-wrap gap-x-3 gap-y-0.5 text-xs text-muted-foreground">
        {present.map((category) => (
          <li key={category} className="flex items-center gap-1">
            <span className={`size-2 rounded-full ${fills[category]}`} aria-hidden />
            {labels(category)} <span className="font-medium text-foreground">{counts[category]}</span>
          </li>
        ))}
        {present.length === 0 && <li>No work item</li>}
      </ul>
    </div>
  )
}
