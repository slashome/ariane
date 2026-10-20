import {
  DndContext,
  KeyboardSensor,
  PointerSensor,
  useDraggable,
  useDroppable,
  useSensor,
  useSensors,
  type DragEndEvent,
} from '@dnd-kit/core'
import { CSS } from '@dnd-kit/utilities'
import type { ReactNode } from 'react'
import { cn } from '@/shared/lib'

export type { DragEndEvent }

export function DragArea({ onDragEnd, children }: { onDragEnd: (event: DragEndEvent) => void; children: ReactNode }) {
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } }),
    useSensor(KeyboardSensor),
  )
  return (
    <DndContext sensors={sensors} onDragEnd={onDragEnd}>
      {children}
    </DndContext>
  )
}

export function Draggable({ id, children, className }: { id: string; children: ReactNode; className?: string }) {
  const { attributes, listeners, setNodeRef, transform, isDragging } = useDraggable({ id })
  return (
    <div
      ref={setNodeRef}
      style={{ transform: CSS.Translate.toString(transform) }}
      className={cn('cursor-grab touch-none active:cursor-grabbing', isDragging && 'relative z-20 opacity-80 shadow-lg', className)}
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
