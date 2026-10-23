import {
  DndContext,
  DragOverlay,
  KeyboardCode,
  KeyboardSensor,
  PointerSensor,
  useDraggable,
  useDroppable,
  useSensor,
  useSensors,
  type DragEndEvent,
} from '@dnd-kit/core'
import { useState, type KeyboardEvent, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { cn } from '@/shared/lib'

export type { DragEndEvent }

// Space picks a card up, so Enter stays free to open it.
const keyboardCodes = {
  start: [KeyboardCode.Space],
  cancel: [KeyboardCode.Esc],
  end: [KeyboardCode.Space, KeyboardCode.Enter],
}

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
    // Below 8 px the press is a click, which opens the card; a shaky click on a trackpad stays one.
    useSensor(PointerSensor, { activationConstraint: { distance: 8 } }),
    useSensor(KeyboardSensor, { keyboardCodes }),
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
// With onOpen, a click or Enter opens it; the click that ends a drag never reaches it.
export function Draggable({
  id,
  children,
  className,
  onOpen,
  label,
}: {
  id: string
  children: ReactNode
  className?: string
  onOpen?: () => void
  label?: string
}) {
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({ id })
  return (
    <div
      ref={setNodeRef}
      className={cn(
        'touch-none active:cursor-grabbing',
        onOpen ? 'cursor-pointer rounded-md hover:ring-1 hover:ring-primary/40' : 'cursor-grab',
        isDragging && 'opacity-40',
        className,
      )}
      {...attributes}
      {...listeners}
      aria-label={label}
      onClick={onOpen}
      onKeyDown={(event: KeyboardEvent<HTMLDivElement>) => {
        // Enter also drops a card carried with the keyboard: only a resting card opens.
        if (onOpen && event.key === 'Enter' && !isDragging) {
          event.preventDefault()
          onOpen()
          return
        }
        listeners?.onKeyDown?.(event)
      }}
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
