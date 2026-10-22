import {
  DndContext,
  DragOverlay,
  KeyboardSensor,
  PointerSensor,
  useDraggable,
  useDroppable,
  useSensor,
  useSensors,
  type DragEndEvent,
} from '@dnd-kit/core'
import { useState, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { cn } from '@/shared/lib'

export type { DragEndEvent }

// The dragged element is drawn by the overlay, portalled to the body, so it floats above
// every column instead of being clipped by the scrolling container it comes from.
export function DragArea({
  onDragEnd,
  renderOverlay,
  children,
}: {
  onDragEnd: (event: DragEndEvent) => void
  renderOverlay?: (id: string) => ReactNode
  children: ReactNode
}) {
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } }),
    useSensor(KeyboardSensor),
  )
  const [active, setActive] = useState<string | null>(null)
  return (
    <DndContext
      sensors={sensors}
      onDragStart={({ active: dragged }) => setActive(String(dragged.id))}
      onDragCancel={() => setActive(null)}
      onDragEnd={(event) => {
        setActive(null)
        onDragEnd(event)
      }}
    >
      {children}
      {renderOverlay &&
        createPortal(
          <DragOverlay dropAnimation={null} className="cursor-grabbing">
            {active === null ? null : renderOverlay(active)}
          </DragOverlay>,
          document.body,
        )}
    </DndContext>
  )
}

// The element stays in place while dragged, dimmed: the overlay of DragArea moves instead.
export function Draggable({ id, children, className }: { id: string; children: ReactNode; className?: string }) {
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({ id })
  return (
    <div
      ref={setNodeRef}
      className={cn('cursor-grab touch-none active:cursor-grabbing', isDragging && 'opacity-40', className)}
      {...attributes}
      {...listeners}
    >
      {children}
    </div>
  )
}

export function DropZone({
  id,
  children,
  className,
  label,
}: {
  id: string
  children: ReactNode
  className?: string
  label?: string
}) {
  const { setNodeRef, isOver } = useDroppable({ id })
  return (
    <div ref={setNodeRef} aria-label={label} className={cn(className, isOver && 'ring-2 ring-primary/60')}>
      {children}
    </div>
  )
}
