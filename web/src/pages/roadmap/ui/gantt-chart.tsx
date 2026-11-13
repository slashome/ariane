import { useMemo, type ReactNode } from 'react'
import { cn } from '@/shared/lib'
import { edgeOf, revealLeft, rowExtent, type Edge } from '../model/edges'
import type { GanttRow } from '../model/rows'
import { ticks } from '../model/scale'
import type { GanttView } from '../model/view'
import { SummaryBars, WorkBars } from './gantt-bars'

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
}: {
  rows: GanttRow[]
  view: GanttView
  measure: (element: HTMLElement | null) => (() => void) | undefined
  now: string
  empty?: ReactNode
  onOpen: (reference: string) => void
}) {
  const nowMs = Date.parse(now)
  const axis = useMemo(
    () => ticks(view.scale, view.dated, nowMs, view.left, view.left + view.width),
    [view.scale, view.dated, view.left, view.width, nowMs],
  )
  const nowX = -view.left
  const placement = { scale: view.scale, left: view.left, width: view.width }

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
              return (
                <li
                  key={row.key}
                  data-level={row.level}
                  className={cn('flex h-7 items-stretch hover:bg-muted/40', row.level === 'release' && 'border-t border-border/70')}
                >
                  <div className="flex w-80 shrink-0 items-center border-r border-border bg-background">
                    <Label row={row} onOpen={open} />
                  </div>
                  <div className="relative min-w-0 flex-1 overflow-hidden">
                    {row.work ? (
                      <WorkBars row={row} work={row.work} placement={placement} dated={view.dated} onOpen={open} />
                    ) : (
                      <SummaryBars row={row} placement={placement} dated={view.dated} now={now} onOpen={open} />
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
