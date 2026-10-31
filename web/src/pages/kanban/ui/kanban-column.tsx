import { categoryTone, ItemCard } from '@/entities/item'
import type { Item } from '@/shared/api'
import { cn } from '@/shared/lib'
import { Draggable, DropZone, Icon } from '@/shared/ui'
import type { KanbanColumn as Column } from '../model/columns'
import './kanban.css'

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
      className={cn('kanban-column', collapsed && 'kanban-column--collapsed')}
    >
      <button
        type="button"
        onClick={onToggle}
        aria-expanded={!collapsed}
        className="kanban-column__header"
      >
        <Icon name={collapsed ? 'chevron-right' : 'chevron-down'} className="kanban-column__chevron" />
        <span className="lu-truncate kanban-column__label">{column.label}</span>
        <span className={cn('kanban-column__count', categoryTone(column.category))}>
          {column.items.length}
        </span>
      </button>
      {!collapsed && (
        <div className="kanban-column__cards">
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
