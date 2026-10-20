import type { Category } from '@/shared/api'
import { Badge } from '@/shared/ui'
import { categoryTone } from '../model/category-tone'

export function StatusBadge({
  status,
  category,
  label,
}: {
  status: string | null
  category: Category | null
  label?: string | null
}) {
  return (
    <Badge className={categoryTone(category)} title={status ?? undefined}>
      {label ?? status ?? 'no status'}
    </Badge>
  )
}
