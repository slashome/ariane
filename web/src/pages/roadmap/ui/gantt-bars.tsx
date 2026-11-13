import type { WorkStage } from '@/shared/api'
import { cn } from '@/shared/lib'
import { Tooltip } from '@/shared/ui'
import { describeFragment, describeWork, hoursLabel } from '../model/describe'
import type { Fragment, GanttRow, Phase, TimedSegment, WorkBar } from '../model/rows'
import type { Scale } from '../model/scale'

const stageFills: Record<WorkStage, string> = {
  open: 'bg-blue-500',
  in_progress: 'bg-amber-500',
  testing: 'bg-amber-500',
  waiting: 'bg-violet-500',
  done: 'bg-green-500',
}

const phaseShades: Record<Phase, string> = {
  work: '',
  test: 'opacity-55',
  validation: 'opacity-25 outline outline-1 -outline-offset-1 outline-dashed outline-foreground/40',
  done: '',
}

const summaryHeights: Record<GanttRow['level'], string> = {
  release: 'h-3',
  epic: 'h-2.5',
  story: 'h-2',
  task: 'h-2',
}

const minimumWidth = 3

interface Placement {
  scale: Scale
  left: number
  width: number
}

function place({ scale, left }: Placement, start: number, end: number) {
  const from = scale.x(start) - left
  return { from, size: Math.max(scale.x(end) - left - from, minimumWidth) }
}

function visible({ width }: Placement, from: number, size: number) {
  return from + size >= -40 && from <= width + 40
}

function Lines({ lines }: { lines: string[] }) {
  return (
    <div className="space-y-0.5 whitespace-pre-line">
      {lines.map((line, index) => (
        <p key={index} className={index === 0 ? 'font-sans font-medium' : undefined}>
          {line}
        </p>
      ))}
    </div>
  )
}

function Trailer({ at, text, missed }: { at: number; text: string; missed: boolean }) {
  if (text === '' && !missed) {
    return null
  }
  return (
    <span className="pointer-events-none absolute top-1/2 flex -translate-y-1/2 items-center gap-1 whitespace-nowrap" style={{ left: at + 4 }}>
      {missed && <span data-testid="missed-target" className="size-1.5 rounded-full bg-red-500" aria-label="target missed" />}
      {text !== '' && <span className="font-mono text-[10px] text-muted-foreground">{text}</span>}
    </span>
  )
}

function Segment({ segment, stage, hatched, approximate, origin, placement }: {
  segment: TimedSegment
  stage: WorkStage
  hatched: boolean
  approximate: boolean
  origin: number
  placement: Placement
}) {
  const { from, size } = place(placement, segment.start, segment.end)
  return (
    <span
      data-phase={segment.phase}
      data-hatched={(hatched && segment.phase !== 'validation') || undefined}
      className={cn(
        'absolute inset-y-0 rounded-[3px]',
        stageFills[stage],
        phaseShades[segment.phase],
        hatched && segment.phase !== 'validation' && 'hatched',
        approximate && 'opacity-60 outline outline-1 -outline-offset-1 outline-dashed outline-foreground/50',
      )}
      style={{ left: from - origin, width: size }}
    />
  )
}

export function WorkBars({
  row,
  work,
  placement,
  dated,
  onOpen,
}: {
  row: GanttRow
  work: WorkBar
  placement: Placement
  dated: boolean
  onOpen?: () => void
}) {
  if (work.segments.length === 0) {
    return null
  }
  const start = Math.min(...work.segments.map((segment) => segment.start))
  const end = Math.max(...work.segments.map((segment) => segment.end))
  const { from, size } = place(placement, start, end)
  if (!visible(placement, from, size)) {
    return null
  }
  const approximate = work.closed?.approximate ?? false
  return (
    <>
      <Tooltip content={<Lines lines={describeWork(row, work, dated)} />}>
        <button
          type="button"
          aria-label={`${row.id ?? row.title} bar`}
          data-stage={work.stage}
          onClick={onOpen}
          className="absolute top-1/2 h-3.5 -translate-y-1/2 rounded-[3px] focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-ring"
          style={{ left: from, width: size }}
        >
          {work.segments.map((segment, index) => (
            <Segment
              key={index}
              segment={segment}
              stage={work.stage}
              hatched={work.hatched}
              approximate={approximate}
              origin={from}
              placement={placement}
            />
          ))}
        </button>
      </Tooltip>
      <Trailer at={from + size} text={work.stage === 'done' ? '' : hoursLabel(work.hours, work.testHours, work.hatched)} missed={row.missed !== null} />
    </>
  )
}

export function SummaryBars({
  row,
  placement,
  dated,
  now,
  onOpen,
}: {
  row: GanttRow
  placement: Placement
  dated: boolean
  now: string
  onOpen?: () => void
}) {
  const last = row.fragments[row.fragments.length - 1]
  return (
    <>
      {row.fragments.map((fragment: Fragment) => {
        const { from, size } = place(placement, fragment.start, fragment.end)
        if (!visible(placement, from, size)) {
          return null
        }
        return (
          <Tooltip key={fragment.release} content={<Lines lines={describeFragment(row, fragment, dated, now)} />}>
            <button
              type="button"
              aria-label={`${row.id ?? row.title} span in ${fragment.release}`}
              data-fragment={fragment.release}
              onClick={onOpen}
              className={cn(
                'absolute top-1/2 -translate-y-1/2 rounded-full bg-muted-foreground/15 focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-ring',
                summaryHeights[row.level],
              )}
              style={{ left: from, width: size }}
            >
              {fragment.pieces.map((piece, index) => {
                const shape = place(placement, piece.start, piece.end)
                return (
                  <span
                    key={index}
                    data-hatched={piece.hatched || undefined}
                    className={cn(
                      'absolute inset-y-0 rounded-full',
                      piece.done ? 'bg-green-500/70' : 'bg-slate-500/80 dark:bg-slate-400/80',
                      piece.hatched && 'hatched',
                    )}
                    style={{ left: shape.from - from, width: shape.size }}
                  />
                )
              })}
            </button>
          </Tooltip>
        )
      })}
      {last && (
        <Trailer
          at={place(placement, last.start, last.end).from + place(placement, last.start, last.end).size}
          text={hoursLabel(row.hours, row.testHours, false)}
          missed={row.missed !== null}
        />
      )}
    </>
  )
}
