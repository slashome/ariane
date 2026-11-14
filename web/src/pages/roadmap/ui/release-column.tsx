import { Ellipsis } from 'lucide-react'
import { useState } from 'react'
import type { Item, Release } from '@/shared/api'
import { cn } from '@/shared/lib'
import { DropZone, Menu } from '@/shared/ui'
import type { ColumnAction, ColumnMenuEntry } from '../model/column-menu'
import { releaseZone, type ColumnSummary, type OutlineEntry } from '../model/release-board'
import { ReleaseOutline } from './release-outline'

function TargetField({ release, editable, onTarget }: { release: Release; editable: boolean; onTarget: (target: string | null) => void }) {
  const [draft, setDraft] = useState(release.target ?? '')
  const [shown, setShown] = useState(release.target)
  if (shown !== release.target) {
    setShown(release.target)
    setDraft(release.target ?? '')
  }
  const commit = () => {
    const next = draft === '' ? null : draft
    if (next !== release.target) {
      onTarget(next)
    }
  }
  return (
    <label className="flex items-center gap-1 text-[11px] text-muted-foreground">
      <span>target</span>
      <input
        type="date"
        aria-label={`Target of ${release.name}`}
        value={draft}
        disabled={!editable}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={commit}
        onKeyDown={(event) => {
          if (event.key === 'Enter') {
            commit()
          }
        }}
        className="h-6 rounded border border-border bg-background px-1 font-mono text-[11px] text-foreground disabled:border-transparent disabled:bg-transparent"
      />
    </label>
  )
}

export function ReleaseColumn({
  release,
  summary,
  entries,
  scope,
  menu,
  editable,
  onAction,
  onTarget,
  onOpen,
}: {
  release: Release
  summary: ColumnSummary
  entries: OutlineEntry[]
  scope: string
  menu: ColumnMenuEntry[]
  editable: boolean
  onAction: (action: ColumnAction) => void
  onTarget: (target: string | null) => void
  onOpen: (item: Item) => void
}) {
  const closed = release.closed ?? null
  if (closed !== null) {
    return (
      <section
        aria-label={`Release ${release.name}`}
        className="flex w-48 shrink-0 flex-col rounded-lg border border-border bg-muted/10 text-muted-foreground"
      >
        <header className="flex flex-col gap-1 border-b border-border px-3 py-2">
          <div className="flex items-center gap-2">
            <h2 className="truncate text-sm font-semibold text-foreground">{release.name}</h2>
            <span className="flex-1" />
            <Menu
              label={`Release ${release.name} actions`}
              trigger={
                <button type="button" className="icon-button size-6" title="Release actions">
                  <Ellipsis className="size-4" aria-hidden />
                </button>
              }
              entries={menu.map((entry) => ({
                key: entry.action,
                label: entry.label,
                disabled: entry.disabled,
                hint: entry.reason,
                destructive: entry.action === 'remove',
                onSelect: () => onAction(entry.action),
              }))}
            />
          </div>
          <p className="text-[11px]">Closed {closed}</p>
          <p className="text-[11px]" aria-label={`Forecast of ${release.name}`}>
            {summary.done} done{summary.open > 0 ? ` · ${summary.open} open` : ''}
          </p>
        </header>
        {entries.length > 0 && (
          <div className="min-h-0 flex-1 overflow-y-auto p-2">
            <ReleaseOutline entries={entries} scope={scope} onOpen={onOpen} empty="" />
          </div>
        )}
      </section>
    )
  }
  return (
    <DropZone
      id={releaseZone(release.name)}
      label={`Release ${release.name}`}
      className={cn('flex w-72 shrink-0 flex-col rounded-lg border border-border bg-muted/30', release.parking && 'border-dashed')}
    >
      <header className="flex flex-col gap-1 border-b border-border px-3 py-2">
        <div className="flex items-center gap-2">
          <h2 className="truncate text-sm font-semibold">{release.name}</h2>
          {release.parking && <span className="rounded bg-muted px-1.5 text-[10px] text-muted-foreground">parking</span>}
          <span className="flex-1" />
          <Menu
            label={`Release ${release.name} actions`}
            trigger={
              <button type="button" className="icon-button size-6" title="Release actions">
                <Ellipsis className="size-4" aria-hidden />
              </button>
            }
            entries={menu.map((entry) => ({
              key: entry.action,
              label: entry.label,
              disabled: entry.disabled,
              hint: entry.reason,
              destructive: entry.action === 'remove',
              onSelect: () => onAction(entry.action),
            }))}
          />
        </div>
        {!release.parking && <TargetField release={release} editable={editable} onTarget={onTarget} />}
        <p className="flex flex-wrap gap-x-2 text-[11px] text-muted-foreground" aria-label={`Forecast of ${release.name}`}>
          {release.parking ? (
            <span>not scheduled</span>
          ) : (
            <>
              {summary.hours !== '' && <span className="font-mono text-foreground">{summary.hours}</span>}
              {summary.end !== null && <span>ends {summary.end}</span>}
            </>
          )}
          <span>
            {summary.done} done · {summary.open} open
          </span>
        </p>
      </header>
      <div className="min-h-24 flex-1 overflow-y-auto p-2">
        <ReleaseOutline entries={entries} scope={scope} onOpen={onOpen} empty="Drop epics, stories or tasks here" />
      </div>
    </DropZone>
  )
}
