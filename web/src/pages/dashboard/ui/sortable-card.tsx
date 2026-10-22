import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { GripVertical, X } from 'lucide-react'
import type { ReactElement } from 'react'

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
    <div className="flex shrink-0 items-center gap-1">
      <button
        type="button"
        className="icon-button size-6 cursor-grab touch-none active:cursor-grabbing"
        aria-label={`Move ${id}`}
        {...attributes}
        {...listeners}
      >
        <GripVertical className="size-4" aria-hidden />
      </button>
      <button type="button" className="icon-button size-6" aria-label={`Unpin ${id}`} onClick={onRemove}>
        <X className="size-4" aria-hidden />
      </button>
    </div>
  )
  return (
    <div
      ref={setNodeRef}
      style={{ transform: CSS.Transform.toString(transform), transition }}
      className={isDragging ? 'relative z-20 opacity-80' : undefined}
    >
      {render(toolbar)}
    </div>
  )
}
