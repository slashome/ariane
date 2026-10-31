import { closestCenter, DndContext, KeyboardSensor, PointerSensor, useSensor, useSensors, type DragEndEvent } from '@dnd-kit/core'
import { SortableContext, sortableKeyboardCoordinates, useSortable, verticalListSortingStrategy } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { ArrowDown, ArrowUp, ArrowUpDown, GripVertical } from 'lucide-react'
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

export interface Reorder<Row> {
  column: string
  onMove: (row: Row, over: Row, place: 'before' | 'after') => void
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
  reorder,
}: {
  rows: Row[]
  columns: Column<Row>[]
  rowKey: (row: Row) => string
  storageKey: string
  label?: string
  rowProps?: (row: Row, index: number) => ComponentProps<'tr'>
  empty?: ReactNode
  className?: string
  reorder?: Reorder<Row>
}) {
  const [sort, setSort] = useStoredSort(storageKey)
  const sortedBy = columns.find((column) => column.id === sort?.column && column.sortValue !== undefined)
  const sorted = useMemo(
    () => (sort && sortedBy?.sortValue ? sortRows(rows, sortedBy.sortValue, sort.direction) : rows),
    [rows, sort, sortedBy],
  )
  const draggable = reorder !== undefined && (sort === null || (sort.column === reorder.column && sort.direction === 'asc'))
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 4 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  )
  const keys = sorted.map(rowKey)
  const onDragEnd = ({ active, over }: DragEndEvent) => {
    const from = keys.indexOf(String(active.id))
    const to = over === null ? -1 : keys.indexOf(String(over.id))
    const [moved, target] = [sorted[from], sorted[to]]
    if (reorder === undefined || moved === undefined || target === undefined || from === to) {
      return
    }
    reorder.onMove(moved, target, from < to ? 'after' : 'before')
  }

  return (
    <div className={cn('overflow-x-auto rounded-lg border border-border', className)}>
      <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={onDragEnd}>
        <SortableContext items={keys} strategy={verticalListSortingStrategy}>
          <table aria-label={label} className="w-full border-collapse text-sm">
            <thead className="bg-muted/50 text-left text-xs text-muted-foreground">
              <tr>
                {reorder !== undefined && (
                  <th scope="col" className="w-6">
                    <span className="sr-only">Move</span>
                  </th>
                )}
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
                const cells = columns.map((column) => (
                  <td key={column.id} className={cn('px-3 py-2', column.className)}>
                    {column.cell(row)}
                  </td>
                ))
                return draggable ? (
                  <SortableRow key={rowKey(row)} id={rowKey(row)} props={props}>
                    {cells}
                  </SortableRow>
                ) : (
                  <tr key={rowKey(row)} {...props} className={cn('border-t border-border align-top', props.className)}>
                    {reorder !== undefined && <td className="w-6" />}
                    {cells}
                  </tr>
                )
              })}
            </tbody>
          </table>
        </SortableContext>
      </DndContext>
      {rows.length === 0 && empty && <p className="p-8 text-center text-sm text-muted-foreground">{empty}</p>}
    </div>
  )
}

function SortableRow({ id, props, children }: { id: string; props: ComponentProps<'tr'>; children: ReactNode }) {
  const { attributes, listeners, setNodeRef, setActivatorNodeRef, transform, transition, isDragging } = useSortable({ id })
  return (
    <tr
      ref={setNodeRef}
      {...props}
      style={{ transform: CSS.Translate.toString(transform), transition }}
      className={cn('border-t border-border align-top', props.className, isDragging && 'relative z-10 bg-background opacity-80')}
    >
      <td className="w-6 py-2 pl-2">
        <button
          type="button"
          ref={setActivatorNodeRef}
          className="icon-button size-6 cursor-grab touch-none text-muted-foreground active:cursor-grabbing"
          aria-label={`Move ${id}`}
          onClick={(event) => event.stopPropagation()}
          {...attributes}
          {...listeners}
        >
          <GripVertical className="size-4" aria-hidden />
        </button>
      </td>
      {children}
    </tr>
  )
}
