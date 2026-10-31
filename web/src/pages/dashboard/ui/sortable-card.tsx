import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import type { ReactElement } from 'react'
import { cn } from '@/shared/lib'
import { Icon } from '@/shared/ui'
import './dashboard.css'

export function SortableCard({
  id,
  onRemove,
  render,
}: {
  id: string
  onRemove: () => void
  render: (toolbar: ReactElement) => ReactElement
}) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id })
  const toolbar = (
    <div className="sortable-card__toolbar">
      <button
        type="button"
        className="lu-icon-button sortable-card__button sortable-card__handle"
        aria-label={`Move ${id}`}
        {...attributes}
        {...listeners}
      >
        <Icon name="grip" className="sortable-card__icon" />
      </button>
      <button type="button" className="lu-icon-button sortable-card__button" aria-label={`Unpin ${id}`} onClick={onRemove}>
        <Icon name="close" className="sortable-card__icon" />
      </button>
    </div>
  )
  return (
    <div
      ref={setNodeRef}
      style={{ transform: CSS.Transform.toString(transform), transition }}
      className={cn('sortable-card', isDragging && 'sortable-card--dragging')}
    >
      {render(toolbar)}
    </div>
  )
}
