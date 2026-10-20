import type { Category } from '@/shared/api'
import { Badge } from '@/shared/ui'
import { categoryTone } from '../model/category-tone'

export function StatusBadge({ status, category }: { status: string | null; category: Category | null }) {
  return <Badge className={categoryTone(category)}>{status ?? 'no status'}</Badge>
}
