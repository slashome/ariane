import type { ReactNode } from 'react'
import type { Category, Item } from '@/shared/api'
import { cn } from '@/shared/lib'
import { Highlight, type Column } from '@/shared/ui'
import { categoryOrder, isClosed } from '../model/categories'
import type { Progress } from '../model/progress'
import { ItemLink } from './item-link'
import { ProgressMeter } from './progress-meter'
import { StatusBadge } from './status-badge'

export function idColumn<Row>(itemOf: (row: Row) => Item): Column<Row> {
  return {
    id: 'id',
    header: 'Id',
    sortValue: (row) => itemOf(row).id,
    className: 'w-px whitespace-nowrap font-mono text-xs text-muted-foreground',
    cell: (row) => <Highlight text={itemOf(row).id ?? '—'} />,
  }
}

export function titleColumn<Row>(
  itemOf: (row: Row) => Item,
  { header = 'Title', details, link = true }: { header?: string; details?: (row: Row) => ReactNode; link?: boolean } = {},
): Column<Row> {
  return {
    id: 'title',
    header,
    sortValue: (row) => itemOf(row).title,
    cell: (row) => {
      const item = itemOf(row)
      const title = <Highlight text={item.title ?? item.path} />
      return (
        <div className={cn('min-w-0', isClosed(item.category) && 'text-muted-foreground')}>
          {link ? (
            <ItemLink item={item} className="font-medium">
              {title}
            </ItemLink>
          ) : (
            <span className="line-clamp-2">{title}</span>
          )}
          {details && <p className="mt-0.5 flex flex-wrap gap-x-3 text-xs text-muted-foreground">{details(row)}</p>}
        </div>
      )
    },
  }
}

export function statusColumn<Row>(itemOf: (row: Row) => Item, labelOf: (item: Item) => string | null): Column<Row> {
  return {
    id: 'status',
    header: 'Status',
    sortValue: (row) => {
      const { category } = itemOf(row)
      return category === null ? null : categoryOrder.indexOf(category)
    },
    className: 'whitespace-nowrap',
    cell: (row) => {
      const item = itemOf(row)
      return <StatusBadge status={item.status} category={item.category} label={labelOf(item)} />
    },
  }
}

export function progressColumn<Row>(
  id: string,
  header: string,
  progressOf: (row: Row) => Progress,
  labels: (category: Category) => string,
  empty?: string,
): Column<Row> {
  return {
    id,
    header,
    sortValue: (row) => {
      const progress = progressOf(row)
      return progress.total === 0 ? null : progress.ratio
    },
    className: 'w-64 min-w-48',
    cell: (row) => <ProgressMeter progress={progressOf(row)} labels={labels} empty={empty} />,
  }
}
