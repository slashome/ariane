import { GripVertical } from 'lucide-react'
import { useMemo, useState, type KeyboardEvent, type PointerEvent, type ReactNode } from 'react'
import type { Item, Pricing } from '@/shared/api'
import { cn } from '@/shared/lib'
import { toast } from '@/shared/ui'
import { edgeOf, revealLeft, rowExtent, type Edge } from '../model/edges'
import { checkDrop, isReorderable, siblingMove, stretchRefusal, type DropCheck, type ItemOf, type Place, type Size } from '../model/edits'
import type { GanttRow } from '../model/rows'
import { ticks } from '../model/scale'
import type { GanttView } from '../model/view'
import { SummaryBars, WorkBars } from './gantt-bars'
import type { Stretch } from './stretch-handle'

export interface GanttEditing {
  itemOf: ItemOf
  pricingOf: (item: Item) => Pricing | undefined
  move: (item: Item, other: Item, place: Place) => void
  resize: (item: Item, size: Size) => void
}

interface Drag {
  row: GanttRow
  over: { key: string; check: DropCheck } | null
}

const indent = 14

function opensDetail(row: GanttRow): boolean {
  return row.reference !== null && (row.kind === 'task' || row.kind === 'story' || row.kind === 'epic')
}

function Label({ row, onOpen }: { row: GanttRow; onOpen?: () => void }) {
  const content = (
    <>
      {row.level === 'release' ? (
        <span className="shrink-0 text-[10px] tracking-wide text-muted-foreground uppercase">Release</span>
      ) : (
        row.id && <span className="shrink-0 font-mono text-[11px] text-muted-foreground">{row.id}</span>
      )}
      <span className={cn('truncate', row.level === 'release' && 'font-semibold', row.level === 'epic' && 'font-medium', row.reference === null && 'italic text-muted-foreground')}>
        {row.title}
      </span>
      {row.missed !== null && <span className="size-1.5 shrink-0 rounded-full bg-red-500" title={`Target missed: ${row.missed}`} aria-label="target missed" />}
    </>
  )
  const style = { paddingLeft: 12 + row.depth * indent }
  return onOpen ? (
    <button type="button" onClick={onOpen} title={row.title} className="flex w-full min-w-0 items-center gap-2 pr-3 text-left text-sm hover:text-primary" style={style}>
      {content}
    </button>
  ) : (
    <div title={row.title} className="flex min-w-0 items-center gap-2 pr-3 text-sm" style={style}>
      {content}
    </div>
  )
}

function Grip({
  row,
  dragging,
  refused,
  onPointerDown,
  onPointerMove,
  onPointerUp,
  onPointerCancel,
}: {
  row: GanttRow
  dragging: boolean
  refused: boolean
  onPointerDown: (event: PointerEvent<HTMLButtonElement>) => void
  onPointerMove: (event: PointerEvent<HTMLButtonElement>) => void
  onPointerUp: () => void
  onPointerCancel: () => void
}) {
  return (
    <button
      type="button"
      aria-label={`Reorder ${row.id ?? row.title}`}
      aria-keyshortcuts="Alt+ArrowUp Alt+ArrowDown"
      title="Drag to reorder, or Alt+↑ / Alt+↓"
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerCancel}
      className={cn(
        'absolute top-1/2 flex h-5 w-3 -translate-y-1/2 touch-none items-center justify-center text-muted-foreground opacity-0 group-hover/row:opacity-100 focus-visible:opacity-100',
        dragging ? 'opacity-100' : 'cursor-grab',
        dragging && (refused ? 'cursor-not-allowed' : 'cursor-grabbing'),
      )}
      style={{ left: row.depth * indent }}
    >
      <GripVertical className="size-3" aria-hidden />
    </button>
  )
}

function DropMark({ drag, row }: { drag: Drag | null; row: GanttRow }) {
  const over = drag?.over
  if (!drag || !over || over.key !== row.key || drag.row.key === row.key) {
    return null
  }
  if (!over.check.ok) {
    return (
      <span data-drop="refused" className="pointer-events-none absolute inset-0 z-20 flex items-center bg-red-500/10 pl-80">
        <span role="status" className="ml-2 rounded bg-red-500 px-1.5 text-[10px] whitespace-nowrap text-white">
          {over.check.reason}
        </span>
      </span>
    )
  }
  return (
    <span
      data-drop={over.check.place}
      className={cn('pointer-events-none absolute inset-x-0 z-20 h-0.5 bg-primary', over.check.place === 'before' ? '-top-px' : '-bottom-px')}
    />
  )
}

function EdgeArrow({ row, edge, onClick }: { row: GanttRow; edge: Edge; onClick: () => void }) {
  const name = row.level === 'release' ? `release ${row.title}` : (row.id ?? row.title)
  const where = edge === 'before' ? 'earlier' : 'later'
  return (
    <button
      type="button"
      data-edge={edge}
      aria-label={`Scroll to ${name}, ${where}`}
      title={`Scroll to ${name}, ${where}`}
      onClick={onClick}
      className={cn(
        'absolute top-1/2 z-10 flex h-5 w-4 -translate-y-1/2 items-center justify-center rounded-sm bg-muted text-xs leading-none text-muted-foreground transition-colors duration-(--duration-hover) ease-(--ease-hover) hover:bg-primary hover:text-primary-foreground focus-visible:outline-2 focus-visible:outline-ring',
        edge === 'before' ? 'left-0.5' : 'right-0.5',
      )}
    >
      {edge === 'before' ? '‹' : '›'}
    </button>
  )
}

export function GanttChart({
  rows,
  view,
  measure,
  now,
  empty,
  onOpen,
  editing,
}: {
  rows: GanttRow[]
  view: GanttView
  measure: (element: HTMLElement | null) => (() => void) | undefined
  now: string
  empty?: ReactNode
  onOpen: (reference: string) => void
  editing?: GanttEditing
}) {
  const [drag, setDrag] = useState<Drag | null>(null)
  const nowMs = Date.parse(now)
  const axis = useMemo(
    () => ticks(view.scale, view.dated, nowMs, view.left, view.left + view.width),
    [view.scale, view.dated, view.left, view.width, nowMs],
  )
  const nowX = -view.left
  const placement = { scale: view.scale, left: view.left, width: view.width }

  const apply = (row: GanttRow, check: DropCheck) => {
    const item = editing?.itemOf(row)
    const other = check.ok ? editing?.itemOf(check.other) : undefined
    if (!check.ok) {
      toast.info(check.reason)
      return
    }
    if (editing && item && other) {
      editing.move(item, other, check.place)
    }
  }

  const pickUp = (row: GanttRow) => (event: PointerEvent<HTMLButtonElement>) => {
    event.preventDefault()
    event.currentTarget.setPointerCapture?.(event.pointerId)
    setDrag({ row, over: null })
  }

  const hover = (event: PointerEvent<HTMLButtonElement>) => {
    if (drag === null || editing === undefined) {
      return
    }
    const target = document.elementFromPoint?.(event.clientX, event.clientY)?.closest<HTMLElement>('[data-row-key]')
    const over = target ? rows.find((entry) => entry.key === target.dataset.rowKey) : undefined
    if (target === null || target === undefined || over === undefined) {
      setDrag({ row: drag.row, over: null })
      return
    }
    const box = target.getBoundingClientRect()
    const place: Place = event.clientY < box.top + box.height / 2 ? 'before' : 'after'
    setDrag({ row: drag.row, over: { key: over.key, check: checkDrop(drag.row, over, place, editing.itemOf) } })
  }

  const drop = () => {
    if (drag?.over && drag.over.key !== drag.row.key) {
      apply(drag.row, drag.over.check)
    }
    setDrag(null)
  }

  const shortcut = (row: GanttRow) => (event: KeyboardEvent<HTMLDivElement>) => {
    if (!editing || !event.altKey || (event.key !== 'ArrowUp' && event.key !== 'ArrowDown')) {
      return
    }
    event.preventDefault()
    apply(row, siblingMove(rows, row, event.key === 'ArrowUp' ? -1 : 1, editing.itemOf))
  }

  const stretchOf = (row: GanttRow): Stretch | undefined => {
    const item = editing?.itemOf(row)
    const pricing = item ? editing?.pricingOf(item) : undefined
    if (!editing || !item || !pricing) {
      return undefined
    }
    return { refusal: stretchRefusal(row, item), size: item.size, pricing, onResize: (size) => editing.resize(item, size) }
  }

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-lg border border-border">
      <div className="flex shrink-0 border-b border-border bg-muted/40">
        <div className="flex w-80 shrink-0 items-end px-3 pb-1 text-xs text-muted-foreground">
          {view.dated ? 'Dates' : 'Past in days · future in focused hours'}
        </div>
        <div ref={measure} data-testid="gantt-axis" className="relative h-8 min-w-0 flex-1 overflow-hidden">
          {axis.map((tick) => (
            <span
              key={`${tick.x}-${tick.label}`}
              className={cn('absolute bottom-1 -translate-x-1/2 font-mono text-[10px] whitespace-nowrap', tick.major ? 'text-foreground' : 'text-muted-foreground')}
              style={{ left: tick.x - view.left }}
            >
              {tick.label}
            </span>
          ))}
          {nowX >= 0 && nowX <= view.width && (
            <span className="absolute top-0.5 -translate-x-1/2 rounded bg-accent px-1 text-[10px] font-medium text-background" style={{ left: nowX }}>
              now
            </span>
          )}
        </div>
      </div>
      <div className="relative min-h-0 flex-1 overflow-y-auto">
        <div className="pointer-events-none absolute inset-y-0 right-0 left-80 overflow-hidden" aria-hidden>
          {axis.map((tick) => (
            <span
              key={`${tick.x}-${tick.label}`}
              className={cn('absolute inset-y-0 w-px', tick.major ? 'bg-border' : 'bg-border/50')}
              style={{ left: tick.x - view.left }}
            />
          ))}
          {nowX >= 0 && nowX <= view.width && <span data-testid="now-line" className="absolute inset-y-0 w-0.5 bg-accent/70" style={{ left: nowX }} />}
        </div>
        {rows.length === 0 ? (
          <div className="relative p-10 text-center text-sm text-muted-foreground">{empty ?? 'Nothing to show.'}</div>
        ) : (
          <ul className="relative" aria-label="Roadmap rows">
            {rows.map((row) => {
              const open = opensDetail(row) && row.reference !== null ? () => onOpen(row.reference as string) : undefined
              const extent = rowExtent(row)
              const edge = edgeOf(extent, placement)
              const reorderable = editing !== undefined && isReorderable(row, editing.itemOf(row))
              const stretch = stretchOf(row)
              return (
                <li
                  key={row.key}
                  data-level={row.level}
                  data-row-key={row.key}
                  className={cn(
                    'group/row relative flex h-7 items-stretch hover:bg-muted/40',
                    row.level === 'release' && 'border-t border-border/70',
                    drag?.row.key === row.key && 'opacity-50',
                  )}
                >
                  <DropMark drag={drag} row={row} />
                  <div className="relative flex w-80 shrink-0 items-center border-r border-border bg-background" onKeyDown={shortcut(row)}>
                    {reorderable && (
                      <Grip
                        row={row}
                        dragging={drag?.row.key === row.key}
                        refused={drag?.over?.check.ok === false && drag.over.key !== row.key}
                        onPointerDown={pickUp(row)}
                        onPointerMove={hover}
                        onPointerUp={drop}
                        onPointerCancel={() => setDrag(null)}
                      />
                    )}
                    <Label row={row} onOpen={open} />
                  </div>
                  <div className="relative min-w-0 flex-1 overflow-hidden">
                    {row.work ? (
                      <WorkBars row={row} work={row.work} placement={placement} dated={view.dated} onOpen={open} stretch={stretch} />
                    ) : (
                      <SummaryBars
                        row={row}
                        placement={placement}
                        dated={view.dated}
                        now={now}
                        onOpen={open}
                        refusal={row.kind === 'story' || row.kind === 'epic' ? stretch?.refusal : null}
                      />
                    )}

                    {edge && extent && <EdgeArrow row={row} edge={edge} onClick={() => view.reveal(revealLeft(extent, edge, placement))} />}
                  </div>
                </li>
              )
            })}
          </ul>
        )}
      </div>
    </div>
  )
}
