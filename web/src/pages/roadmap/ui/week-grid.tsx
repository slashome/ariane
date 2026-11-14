import { X } from 'lucide-react'
import { useRef, useState, type KeyboardEvent, type PointerEvent } from 'react'
import { cn, displayNode } from '@/shared/lib'
import { Tooltip } from '@/shared/ui'
import type { Refused } from '../model/calendar-draft'
import {
  dayMinutes,
  describeBlock,
  drawnBlock,
  formatTime,
  movedBlock,
  overlapsOf,
  replaced,
  resizedBlock,
  snapMinutes,
  weekdays,
  type Block,
  type GridPoint,
  type NodeBlocks,
} from '../model/week-grid'

type Gesture =
  | { kind: 'draw'; start: GridPoint; end: GridPoint }
  | { kind: 'move'; index: number; origin: GridPoint; at: GridPoint }
  | { kind: 'resize'; index: number; edge: 'from' | 'to'; minute: number }

const hourHeight = 20
const gridHeight = 24 * hourHeight
const dragThreshold = 3

function previewOf(blocks: Block[], gesture: Gesture | null): Block[] {
  if (gesture === null) {
    return blocks
  }
  switch (gesture.kind) {
    case 'draw':
      return [...blocks, drawnBlock(gesture.start, gesture.end)]
    case 'move': {
      const block = blocks[gesture.index]
      return block === undefined
        ? blocks
        : replaced(blocks, gesture.index, movedBlock(block, gesture.at.day - gesture.origin.day, gesture.at.minute - gesture.origin.minute))
    }
    case 'resize': {
      const block = blocks[gesture.index]
      return block === undefined ? blocks : replaced(blocks, gesture.index, resizedBlock(block, gesture.edge, gesture.minute))
    }
  }
}

function segmentStyle(block: Block, day: number) {
  return {
    top: (block.from / dayMinutes) * gridHeight,
    height: ((block.to - block.from) / dayMinutes) * gridHeight,
    left: `calc(${(day * 100) / 7}% + 2px)`,
    width: `calc(${100 / 7}% - 4px)`,
  }
}

function keyedChange(block: Block, event: KeyboardEvent): Block | null | undefined {
  if (event.key === 'Delete' || event.key === 'Backspace') {
    return null
  }
  const steps: Record<string, [number, number]> = { ArrowUp: [0, -snapMinutes], ArrowDown: [0, snapMinutes], ArrowLeft: [-1, 0], ArrowRight: [1, 0] }
  const step = steps[event.key]
  if (step === undefined) {
    return undefined
  }
  if (event.shiftKey && step[0] === 0) {
    return resizedBlock(block, 'to', block.to + step[1])
  }
  return movedBlock(block, step[0], step[1])
}

export function WeekGrid({
  node,
  blocks,
  others,
  carrier,
  editable,
  refused,
  onChange,
}: {
  node: string
  blocks: Block[]
  others: NodeBlocks[]
  carrier: string | null
  editable: boolean
  refused: Refused | null
  onChange: (blocks: Block[]) => void
}) {
  const area = useRef<HTMLDivElement>(null)
  const [gesture, setGesture] = useState<Gesture | null>(null)
  const [moved, setMoved] = useState(false)
  const shown = previewOf(blocks, gesture)
  const overlaps = overlapsOf(shown, others)
  const clashing = new Set(overlaps.map((overlap) => overlap.block))
  const clashingOthers = new Set(overlaps.filter((overlap) => overlap.other !== null).map((overlap) => `${overlap.other}#${overlap.otherBlock}`))
  for (const overlap of overlaps.filter((entry) => entry.other === null)) {
    clashing.add(overlap.otherBlock)
  }
  const refusedOwn = new Set(refused?.own ?? [])
  const refusedOthers = new Set((refused?.others ?? []).map((entry) => `${entry.node}#${entry.block}`))

  const pointAt = (event: PointerEvent): GridPoint => {
    const rect = area.current?.getBoundingClientRect()
    if (!rect || rect.width === 0 || rect.height === 0) {
      return { day: 0, minute: 0 }
    }
    const day = Math.min(Math.max(Math.floor(((event.clientX - rect.left) / rect.width) * 7), 0), 6)
    const minute = Math.min(Math.max(((event.clientY - rect.top) / rect.height) * dayMinutes, 0), dayMinutes)
    return { day, minute }
  }

  const begin = (event: PointerEvent, next: Gesture) => {
    if (!editable) {
      return
    }
    event.preventDefault()
    event.stopPropagation()
    area.current?.setPointerCapture?.(event.pointerId)
    setMoved(false)
    setGesture(next)
  }

  const onAreaDown = (event: PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) {
      return
    }
    const at = pointAt(event)
    begin(event, { kind: 'draw', start: at, end: at })
  }

  const onAreaMove = (event: PointerEvent<HTMLDivElement>) => {
    if (gesture === null) {
      return
    }
    const at = pointAt(event)
    setMoved(true)
    switch (gesture.kind) {
      case 'draw':
        setGesture({ ...gesture, end: at })
        return
      case 'move':
        setGesture({ ...gesture, at })
        return
      case 'resize':
        setGesture({ ...gesture, minute: at.minute })
        return
    }
  }

  const onAreaUp = (event: PointerEvent<HTMLDivElement>) => {
    if (gesture === null) {
      return
    }
    const at = pointAt(event)
    const far =
      gesture.kind !== 'draw' || Math.abs(at.minute - gesture.start.minute) * (gridHeight / dayMinutes) >= dragThreshold || at.day !== gesture.start.day
    if (moved && far) {
      onChange(previewOf(blocks, gesture))
    }
    setGesture(null)
  }

  return (
    <div className="flex flex-col gap-1">
      <div className="ml-12 grid grid-cols-7 text-center text-xs font-medium text-muted-foreground">
        {weekdays.map((day) => (
          <span key={day} className="capitalize">
            {day}
          </span>
        ))}
      </div>
      <div className="flex">
        <div className="relative w-12 shrink-0" style={{ height: gridHeight }} aria-hidden>
          {Array.from({ length: 12 }, (_, index) => (
            <span key={index} className="absolute right-2 -translate-y-1/2 font-mono text-[10px] text-muted-foreground" style={{ top: index * 2 * hourHeight }}>
              {String(index * 2).padStart(2, '0')}:00
            </span>
          ))}
        </div>
        <div
          ref={area}
          role="group"
          aria-label="Week"
          data-testid="week-area"
          onPointerDown={onAreaDown}
          onPointerMove={onAreaMove}
          onPointerUp={onAreaUp}
          onPointerCancel={() => setGesture(null)}
          className={cn(
            'relative flex-1 touch-none overflow-hidden rounded-md border border-border select-none',
            editable ? 'cursor-crosshair' : 'cursor-default',
          )}
          style={{
            height: gridHeight,
            backgroundImage: `linear-gradient(to bottom, var(--color-border) 1px, transparent 1px), linear-gradient(to right, var(--color-border) 1px, transparent 1px)`,
            backgroundSize: `100% ${hourHeight}px, ${100 / 7}% 100%`,
          }}
        >
          {others.flatMap(({ node: owner, blocks: theirs }) =>
            theirs.flatMap((block, index) =>
              block.days.map((day) => {
                const key = `${owner}#${index}`
                const conflict = clashingOthers.has(key) || refusedOthers.has(key)
                const isCarrier = owner === carrier
                return (
                  <Tooltip key={`${key}-${day}`} content={`${displayNode(owner)} · ${describeBlock(block)}`}>
                    <div
                      tabIndex={0}
                      role="img"
                      aria-label={`${displayNode(owner)}: ${describeBlock(block)}`}
                      data-testid="other-block"
                      data-refused={refusedOthers.has(key) || undefined}
                      className={cn(
                        'absolute overflow-hidden rounded-sm border px-1 text-[10px] leading-tight',
                        isCarrier ? 'border-primary/40 bg-primary/15 text-foreground' : 'border-border bg-muted text-muted-foreground',
                        conflict && 'border-red-500 ring-2 ring-red-500/60',
                      )}
                      style={segmentStyle(block, weekdays.indexOf(day))}
                    >
                      {displayNode(owner)}
                    </div>
                  </Tooltip>
                )
              }),
            ),
          )}
          {shown.flatMap((block, index) =>
            block.days.map((day) => {
              const conflict = clashing.has(index) || refusedOwn.has(index)
              return (
                <div
                  key={`${index}-${day}`}
                  role="button"
                  tabIndex={editable ? 0 : -1}
                  aria-label={`${describeBlock(block)}, block ${index + 1}`}
                  data-testid="own-block"
                  data-refused={refusedOwn.has(index) || undefined}
                  data-overlap={clashing.has(index) || undefined}
                  onPointerDown={(event) => {
                    const at = pointAt(event)
                    begin(event, { kind: 'move', index, origin: at, at })
                  }}
                  onKeyDown={(event) => {
                    const change = keyedChange(block, event)
                    if (change !== undefined && editable) {
                      event.preventDefault()
                      onChange(replaced(blocks, index, change))
                    }
                  }}
                  className={cn(
                    'group absolute rounded-sm border border-primary bg-primary/80 px-1 text-[10px] leading-tight text-primary-foreground focus-visible:outline-2 focus-visible:outline-ring',
                    editable && 'cursor-move',
                    conflict && 'border-red-600 bg-red-500/80 ring-2 ring-red-500',
                  )}
                  style={segmentStyle(block, weekdays.indexOf(day))}
                >
                  <span className="pointer-events-none block truncate">{`${formatTime(block.from)}–${formatTime(block.to)}`}</span>
                  {editable && (
                    <>
                      <span
                        aria-hidden
                        className="absolute inset-x-0 top-0 h-1.5 cursor-ns-resize"
                        onPointerDown={(event) => begin(event, { kind: 'resize', index, edge: 'from', minute: block.from })}
                      />
                      <span
                        aria-hidden
                        className="absolute inset-x-0 bottom-0 h-1.5 cursor-ns-resize"
                        onPointerDown={(event) => begin(event, { kind: 'resize', index, edge: 'to', minute: block.to })}
                      />
                      <button
                        type="button"
                        aria-label={`Delete ${describeBlock(block)}`}
                        onPointerDown={(event) => event.stopPropagation()}
                        onClick={() => onChange(replaced(blocks, index, null))}
                        className="absolute top-0.5 right-0.5 hidden rounded-sm bg-background/80 text-foreground group-hover:block group-focus-visible:block"
                      >
                        <X className="size-3" aria-hidden />
                      </button>
                    </>
                  )}
                </div>
              )
            }),
          )}
          {!editable && shown.length === 0 && others.length === 0 && (
            <p className="absolute inset-0 flex items-center justify-center text-xs text-muted-foreground">No blocks</p>
          )}
        </div>
      </div>
      {editable && (
        <p className="ml-12 text-xs text-muted-foreground">
          Draw on {displayNode(node)}'s week to add a block, drag one to move it, its edges to resize it; arrows move a focused block, Shift+↑↓ resize it,
          Delete removes it. Times snap to 30 minutes.
        </p>
      )}
    </div>
  )
}
