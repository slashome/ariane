import { CalendarPlus, Info, Save, TriangleAlert } from 'lucide-react'
import { useMemo, useState } from 'react'
import { Link, useLocation } from 'react-router'
import { ApiError, type Calendar, type CalendarOverview, type CalendarProblem, type CalendarRefusal } from '@/shared/api'
import { searchParamKeys } from '@/shared/config'
import { cn, displayNode } from '@/shared/lib'
import { Button, ErrorState, Input, LoadingState, NativeSelect } from '@/shared/ui'
import { useCalendar, useCreateCalendar, useSaveCalendar } from '../api/use-calendar'
import {
  carrierOf,
  defaultFocus,
  formatDayOff,
  isWarning,
  minFocus,
  otherBlocks,
  refusedBlocks,
  requestsFor,
  sameDraft,
  savedDraft,
  type CalendarDraft,
  type NodeDraft,
  type Refused,
} from '../model/calendar-draft'
import { describeBlock, focusSummary, fromCalendarBlock, overlapsOf, weeklyHours, type Block } from '../model/week-grid'
import { BlockList } from './block-list'
import { DaysOffList } from './days-off-list'
import { WeekGrid } from './week-grid'

const emptyNode: NodeDraft = { blocks: [], focus: defaultFocus, daysOff: [] }

function browserTimezone(): string {
  return Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC'
}

function timezones(current: string): string[] {
  const known = typeof Intl.supportedValuesOf === 'function' ? Intl.supportedValuesOf('timeZone') : []
  return known.includes(current) ? known : [current, ...known]
}

function TimezoneSelect({ value, onChange }: { value: string; onChange: (timezone: string) => void }) {
  return (
    <NativeSelect aria-label="Time zone" value={value} onChange={(event) => onChange(event.target.value)} className="w-64">
      {timezones(value).map((zone) => (
        <option key={zone} value={zone}>
          {zone}
        </option>
      ))}
    </NativeSelect>
  )
}

function CreateCalendar({ file }: { file: string }) {
  const [timezone, setTimezone] = useState(browserTimezone)
  const create = useCreateCalendar()
  return (
    <div className="flex flex-col items-center gap-3 rounded-lg border border-dashed border-border p-8 text-center text-sm text-muted-foreground">
      <CalendarPlus className="size-6" aria-hidden />
      <p className="font-medium text-foreground">No work calendar yet</p>
      <p className="max-w-lg">
        Without one the roadmap has no dates. It is created at <code className="font-mono text-xs">{file}</code>, never shared with the team.
      </p>
      <div className="flex flex-wrap items-center justify-center gap-2">
        <TimezoneSelect value={timezone} onChange={setTimezone} />
        <Button disabled={create.isPending} onClick={() => create.mutate(timezone)}>
          <CalendarPlus className="size-4" aria-hidden />
          Create my calendar
        </Button>
      </div>
    </div>
  )
}

function Problems({ problems }: { problems: CalendarProblem[] }) {
  if (problems.length === 0) {
    return null
  }
  return (
    <ul aria-label="Calendar problems" className="space-y-1">
      {problems.map((problem, index) => (
        <li
          key={index}
          className={cn(
            'flex items-start gap-1.5 text-xs',
            isWarning(problem) ? 'text-amber-700 dark:text-amber-300' : 'text-red-700 dark:text-red-300',
          )}
        >
          <TriangleAlert className="mt-px size-3.5 shrink-0" aria-hidden />
          {problem.message}
        </li>
      ))}
    </ul>
  )
}

function CarrierNotice({ node, carrier, onOwn }: { node: string; carrier: string; onOwn: () => void }) {
  const { pathname, search } = useLocation()
  const params = new URLSearchParams(search)
  params.set(searchParamKeys.node, carrier)
  return (
    <div
      role="note"
      aria-label="Inherited hours"
      className="flex flex-wrap items-center gap-x-2 gap-y-1 rounded-lg border border-border bg-muted/40 px-3 py-1.5 text-xs text-muted-foreground"
    >
      <Info className="size-4 shrink-0" aria-hidden />
      <span>
        Your hours for {displayNode(node)} come from {displayNode(carrier)} — shared with its sub-nodes.
      </span>
      <Link to={{ pathname, search: `?${params.toString()}` }} className="font-medium text-foreground underline underline-offset-2 hover:text-primary">
        Open {displayNode(carrier)}
      </Link>
      <Button size="sm" variant="outline" className="ml-auto h-7" onClick={onOwn}>
        Give {displayNode(node)} its own blocks
      </Button>
    </div>
  )
}

function FocusField({ value, onChange }: { value: number; onChange: (focus: number) => void }) {
  return (
    <div className="flex flex-wrap items-center gap-3">
      <label htmlFor="focus" className="text-sm font-medium">
        Focus
      </label>
      <input
        id="focus"
        type="range"
        min={minFocus}
        max={1}
        step={0.05}
        value={value}
        onChange={(event) => onChange(Number(event.target.value))}
        className="w-48 accent-(--color-primary)"
      />
      <Input
        type="number"
        aria-label="Focus value"
        min={minFocus}
        max={1}
        step={0.05}
        value={value}
        onChange={(event) => {
          const next = Number(event.target.value)
          if (Number.isFinite(next) && next >= minFocus && next <= 1) {
            onChange(next)
          }
        }}
        className="h-8 w-20"
      />
    </div>
  )
}

function SaveBar({ pending, onSave, onDiscard }: { pending: boolean; onSave: () => void; onDiscard: () => void }) {
  return (
    <div
      role="region"
      aria-label="Unsaved calendar"
      className="sticky bottom-0 z-20 flex flex-wrap items-center gap-2 rounded-lg border border-primary/40 bg-background/95 px-3 py-2 shadow-md backdrop-blur"
    >
      <span className="text-sm">
        Unsaved changes to <code className="font-mono text-xs">calendar.toml</code>
      </span>
      <Button variant="ghost" size="sm" className="ml-auto" disabled={pending} onClick={onDiscard}>
        Discard
      </Button>
      <Button size="sm" disabled={pending} onClick={onSave}>
        <Save className="size-4" aria-hidden />
        {pending ? 'Saving…' : 'Save'}
      </Button>
    </div>
  )
}

function carrierBlocks(calendar: Calendar, carrier: string | null): Block[] {
  return carrier === null ? [] : (calendar.nodes[carrier]?.blocks ?? []).map(fromCalendarBlock)
}

function NodeHours({
  node,
  calendar,
  draft,
  refused,
  onNode,
}: {
  node: string
  calendar: Calendar
  draft: CalendarDraft
  refused: Refused | null
  onNode: (next: NodeDraft | null) => void
}) {
  const carrier = carrierOf(calendar, node)
  const inheritedFrom = carrier !== null && carrier !== node ? carrier : null
  const own = draft.node
  const editable = own !== null || inheritedFrom === null
  const others = useMemo(() => otherBlocks(calendar, node), [calendar, node])
  const blocks = own?.blocks ?? []
  const overlaps = overlapsOf(blocks, others)
  const overlapping = new Set(overlaps.flatMap((overlap) => (overlap.other === null ? [overlap.block, overlap.otherBlock] : [overlap.block])))
  const hours = own !== null || inheritedFrom === null ? weeklyHours(blocks) : weeklyHours(carrierBlocks(calendar, inheritedFrom))
  const carrierTable = inheritedFrom === null ? undefined : calendar.nodes[inheritedFrom]
  const focus = own?.focus ?? carrierTable?.focus ?? defaultFocus
  const change = (next: Partial<NodeDraft>) => onNode({ ...(own ?? emptyNode), ...next })

  return (
    <div className="space-y-4">
      {inheritedFrom !== null && own === null && <CarrierNotice node={node} carrier={inheritedFrom} onOwn={() => onNode(emptyNode)} />}
      {inheritedFrom !== null && own !== null && (
        <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
          <Info className="size-4 shrink-0" aria-hidden />
          {displayNode(node)} has its own blocks; without any it shares the hours of {displayNode(inheritedFrom)}.
          <Button size="sm" variant="ghost" className="h-7" onClick={() => onNode(null)}>
            Use {displayNode(inheritedFrom)}'s hours
          </Button>
        </div>
      )}
      <WeekGrid
        node={node}
        blocks={blocks}
        others={others}
        carrier={own === null ? inheritedFrom : null}
        editable={editable}
        refused={refused}
        onChange={(next) => change({ blocks: next })}
      />
      {overlaps.length > 0 && (
        <p role="alert" className="flex items-start gap-1.5 text-xs text-red-700 dark:text-red-300">
          <TriangleAlert className="mt-px size-3.5 shrink-0" aria-hidden />
          {overlaps
            .map((overlap) => {
              const other = overlap.other === null ? blocks[overlap.otherBlock] : others.find((entry) => entry.node === overlap.other)?.blocks[overlap.otherBlock]
              const mine = blocks[overlap.block]
              return `${mine ? describeBlock(mine) : ''} overlaps ${overlap.other === null ? '' : `${displayNode(overlap.other)} `}${other ? describeBlock(other) : ''}`
            })
            .join('; ')}
          . An hour counts once: saving is refused until they no longer share time.
        </p>
      )}
      {own !== null && <BlockList blocks={blocks} overlapping={overlapping} onChange={(next) => change({ blocks: next })} />}
      {own !== null ? (
        <FocusField value={focus} onChange={(next) => change({ focus: next })} />
      ) : (
        <p className="text-sm">
          <span className="font-medium">Focus</span> <span className="font-mono">{focus}</span>
          {inheritedFrom !== null && <span className="text-xs text-muted-foreground"> · from {displayNode(inheritedFrom)}</span>}
        </p>
      )}
      <p className="font-mono text-xs text-muted-foreground" data-testid="focus-summary">
        {focusSummary(hours, focus)}
      </p>
      <DaysOffList
        label={`Days off of ${displayNode(own === null && inheritedFrom !== null ? inheritedFrom : node)}`}
        days={own?.daysOff ?? (carrierTable?.days_off ?? []).map(formatDayOff)}
        editable={own !== null}
        onChange={(daysOff) => change({ daysOff })}
      />
    </div>
  )
}

function CalendarEditor({ overview, calendar, node }: { overview: CalendarOverview; calendar: Calendar; node: string | null }) {
  const saved = useMemo(() => savedDraft(calendar, node), [calendar, node])
  const [draft, setDraft] = useState<CalendarDraft | null>(null)
  const [refused, setRefused] = useState<Refused | null>(null)
  const save = useSaveCalendar()
  const current = draft ?? saved
  const dirty = draft !== null && !sameDraft(saved, draft)

  const edit = (next: CalendarDraft) => {
    setDraft(next)
    setRefused(null)
  }
  const discard = () => {
    setDraft(null)
    setRefused(null)
  }
  const submit = () => {
    save.mutate(requestsFor(saved, current, node), {
      onSuccess: () => setDraft(null),
      onError: (error) => {
        const problems = error instanceof ApiError && error.status === 422 ? ((error.body as CalendarRefusal | null)?.problems ?? []) : []
        setRefused(node === null ? null : refusedBlocks(problems, node))
      },
    })
  }

  return (
    <div className="space-y-6">
      <Problems problems={overview.problems} />
      {node === null ? (
        <p className="text-sm text-muted-foreground">Choose a node in the header to set its blocks, focus and days off.</p>
      ) : (
        <NodeHours node={node} calendar={calendar} draft={current} refused={refused} onNode={(next) => edit({ ...current, node: next })} />
      )}
      <div className="space-y-4 border-t border-border pt-4">
        <h3 className="text-sm font-semibold">Every node</h3>
        <div className="flex flex-wrap items-center gap-3">
          <span className="text-sm font-medium">Time zone</span>
          <TimezoneSelect value={current.timezone} onChange={(timezone) => edit({ ...current, timezone })} />
          <span className="text-xs text-muted-foreground">Every block is written in it.</span>
        </div>
        <DaysOffList label="Days off of every node" days={current.daysOff} editable onChange={(daysOff) => edit({ ...current, daysOff })} />
      </div>
      {dirty && <SaveBar pending={save.isPending} onSave={submit} onDiscard={discard} />}
    </div>
  )
}

export function CalendarSection({ node }: { node: string | null }) {
  const { data: overview, isPending, error } = useCalendar()
  if (error) {
    return <ErrorState message={`Could not read the calendar: ${error.message}`} />
  }
  if (isPending) {
    return <LoadingState rows={3} />
  }
  if (!overview.present) {
    return <CreateCalendar file={overview.file} />
  }
  if (overview.calendar === null) {
    return (
      <div className="space-y-2">
        <ErrorState message="calendar.toml cannot be read: fix it with `ariane calendar edit`." />
        <Problems problems={overview.problems} />
      </div>
    )
  }
  return <CalendarEditor key={node ?? ''} overview={overview} calendar={overview.calendar} node={node} />
}
