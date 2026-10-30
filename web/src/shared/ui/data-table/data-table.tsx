import { ArrowDown, ArrowUp, ArrowUpDown } from 'lucide-react'
import { useMemo, type ComponentProps, type ReactNode } from 'react'
import { cn } from '@/shared/lib'
import { nextSort, sortRows, type SortDirection, type SortValue } from './sort'
import { useStoredSort } from './use-stored-sort'

export interface Column<Row> {
  id: string
  header: ReactNode
  cell: (row: Row) => ReactNode
  sortValue?: (row: Row) => SortValue
  className?: string
  headerClassName?: string
}

function ariaSort(direction: SortDirection | undefined) {
  return direction === 'asc' ? 'ascending' : direction === 'desc' ? 'descending' : 'none'
}

export function DataTable<Row>({
  rows,
  columns,
  rowKey,
  storageKey,
  label,
  rowProps,
  empty,
  className,
}: {
  rows: Row[]
  columns: Column<Row>[]
  rowKey: (row: Row) => string
  storageKey: string
  label?: string
  rowProps?: (row: Row, index: number) => ComponentProps<'tr'>
  empty?: ReactNode
  className?: string
}) {
  const [sort, setSort] = useStoredSort(storageKey)
  const sortedBy = columns.find((column) => column.id === sort?.column && column.sortValue !== undefined)
  const sorted = useMemo(
    () => (sort && sortedBy?.sortValue ? sortRows(rows, sortedBy.sortValue, sort.direction) : rows),
    [rows, sort, sortedBy],
  )

  return (
    <div className={cn('overflow-x-auto rounded-lg border border-border', className)}>
      <table aria-label={label} className="w-full border-collapse text-sm">
        <thead className="bg-muted/50 text-left text-xs text-muted-foreground">
          <tr>
            {columns.map((column) => {
              const direction = sortedBy?.id === column.id ? sort?.direction : undefined
              const Icon = direction === 'asc' ? ArrowUp : direction === 'desc' ? ArrowDown : ArrowUpDown
              return (
                <th
                  key={column.id}
                  scope="col"
                  aria-sort={column.sortValue ? ariaSort(direction) : undefined}
                  className={cn('px-3 py-2 font-medium', column.headerClassName)}
                >
                  {column.sortValue ? (
                    <button
                      type="button"
                      className={cn('inline-flex items-center gap-1 hover:text-foreground', direction && 'text-foreground')}
                      onClick={() => setSort(nextSort(sort, column.id))}
                    >
                      {column.header}
                      <Icon className={cn('size-3', !direction && 'opacity-40')} aria-hidden />
                    </button>
                  ) : (
                    column.header
                  )}
                </th>
              )
            })}
          </tr>
        </thead>
        <tbody>
          {sorted.map((row, index) => {
            const props = rowProps?.(row, index) ?? {}
            return (
              <tr key={rowKey(row)} {...props} className={cn('border-t border-border align-top', props.className)}>
                {columns.map((column) => (
                  <td key={column.id} className={cn('px-3 py-2', column.className)}>
                    {column.cell(row)}
                  </td>
                ))}
              </tr>
            )
          })}
        </tbody>
      </table>
      {rows.length === 0 && empty && <p className="p-8 text-center text-sm text-muted-foreground">{empty}</p>}
    </div>
  )
}
