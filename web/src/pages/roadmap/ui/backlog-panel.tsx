import type { ReactNode } from 'react'
import type { Kind } from '@/shared/api'
import { cn } from '@/shared/lib'
import { DropZone, Input, NativeSelect } from '@/shared/ui'
import { backlogZone, planningKinds, type BacklogFilters, type BacklogShow } from '../model/release-board'

const shows: { value: BacklogShow; label: string }[] = [
  { value: 'unplanned', label: 'Unplanned' },
  { value: 'all', label: 'All open' },
]

function Toggle({ pressed, onClick, children }: { pressed: boolean; onClick: () => void; children: string }) {
  return (
    <button
      type="button"
      aria-pressed={pressed}
      onClick={onClick}
      className={cn(
        'px-2 py-0.5 text-[11px] text-muted-foreground hover:bg-muted',
        pressed && 'bg-primary text-primary-foreground hover:bg-primary',
      )}
    >
      {children}
    </button>
  )
}

function toggled(kinds: Kind[], kind: Kind): Kind[] {
  return kinds.includes(kind) ? kinds.filter((entry) => entry !== kind) : [...kinds, kind]
}

export function BacklogPanel({
  filters,
  types,
  count,
  onFilters,
  children,
}: {
  filters: BacklogFilters
  types: string[]
  count: number
  onFilters: (filters: BacklogFilters) => void
  children: ReactNode
}) {
  return (
    <DropZone id={backlogZone} label="Backlog" className="flex w-80 shrink-0 flex-col rounded-lg border border-border bg-background">
      <header className="flex flex-col gap-2 border-b border-border px-3 py-2">
        <div className="flex items-center gap-2">
          <h2 className="text-sm font-semibold">Backlog</h2>
          <span className="font-mono text-[11px] text-muted-foreground">{count}</span>
          <span className="flex-1" />
          <div role="group" aria-label="Show" className="inline-flex overflow-hidden rounded-md border border-border">
            {shows.map((show) => (
              <Toggle key={show.value} pressed={filters.show === show.value} onClick={() => onFilters({ ...filters, show: show.value })}>
                {show.label}
              </Toggle>
            ))}
          </div>
        </div>
        <div className="flex items-center gap-2">
          <div role="group" aria-label="Levels" className="inline-flex overflow-hidden rounded-md border border-border">
            {planningKinds.map((kind) => (
              <Toggle key={kind} pressed={filters.kinds.includes(kind)} onClick={() => onFilters({ ...filters, kinds: toggled(filters.kinds, kind) })}>
                {kind}
              </Toggle>
            ))}
          </div>
          <NativeSelect
            aria-label="Type"
            value={filters.type ?? ''}
            onChange={(event) => onFilters({ ...filters, type: event.target.value === '' ? null : event.target.value })}
            className="h-7 text-xs"
          >
            <option value="">All types</option>
            {types.map((type) => (
              <option key={type} value={type}>
                {type}
              </option>
            ))}
          </NativeSelect>
        </div>
        <Input
          type="search"
          aria-label="Filter the backlog"
          placeholder="Filter by id or title"
          value={filters.text}
          onChange={(event) => onFilters({ ...filters, text: event.target.value })}
          className="h-7 text-xs"
        />
      </header>
      <div className="min-h-24 flex-1 overflow-y-auto p-2">{children}</div>
    </DropZone>
  )
}
