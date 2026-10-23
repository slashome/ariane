import { ChevronDown, ChevronRight } from 'lucide-react'
import { categoryTone, ItemCard } from '@/entities/item'
import type { Item } from '@/shared/api'
import { cn } from '@/shared/lib'
import { Draggable, DropZone } from '@/shared/ui'
import type { KanbanColumn as Column } from '../model/columns'

export function KanbanColumn({
  column,
  collapsed,
  scope,
  onToggle,
  onOpen,
  selected,
  waitingOf,
}: {
  column: Column
  collapsed: boolean
  scope: string | null
  onToggle: () => void
  onOpen: (item: Item) => void
  // Path of the item in the preview panel.
  selected: string | null
  waitingOf: (item: Item) => string | undefined
}) {
  return (
    <DropZone
      id={column.id}
      label={column.label}
      className={cn(
        'flex shrink-0 flex-col rounded-lg border border-border bg-muted/30 transition-[width]',
        collapsed ? 'w-12' : 'w-72',
      )}
    >
      <button
        type="button"
        onClick={onToggle}
        aria-expanded={!collapsed}
        className={cn(
          'flex items-center gap-2 border-b border-border px-3 py-2 text-left text-sm font-medium',
          collapsed && 'h-full flex-col border-b-0 px-0 py-3',
        )}
      >
        {collapsed ? <ChevronRight className="size-4" aria-hidden /> : <ChevronDown className="size-4" aria-hidden />}
        <span className={cn('truncate', collapsed && '[writing-mode:vertical-rl]')}>{column.label}</span>
        <span className={cn('ml-auto rounded-full px-1.5 text-xs', categoryTone(column.category), collapsed && 'ml-0')}>
          {column.items.length}
        </span>
      </button>
      {!collapsed && (
        <div className="flex min-h-24 flex-1 flex-col gap-2 overflow-y-auto p-2">
          {column.items.map((item) => (
            <Draggable
              key={item.path}
              id={item.path}
              label={`Open ${item.title ?? item.path}`}
              onOpen={() => onOpen(item)}
            >
              <ItemCard
                item={item}
                showNode={item.node !== scope}
                selected={item.path === selected}
                waiting={waitingOf(item)}
              />
            </Draggable>
          ))}
        </div>
      )}
    </DropZone>
  )
}
