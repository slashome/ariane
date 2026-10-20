import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { GripVertical, X } from 'lucide-react'
import type { ReactElement } from 'react'
import { Button } from '@/shared/ui'

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
        className="cursor-grab touch-none rounded p-1 text-muted-foreground hover:bg-muted active:cursor-grabbing"
        aria-label={`Move ${id}`}
        {...attributes}
        {...listeners}
      >
        <GripVertical className="size-4" aria-hidden />
      </button>
      <Button size="icon" variant="ghost" aria-label={`Unpin ${id}`} onClick={onRemove}>
        <X className="size-4" aria-hidden />
      </Button>
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
