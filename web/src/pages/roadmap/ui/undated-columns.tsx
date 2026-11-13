import { ChevronsRight } from 'lucide-react'
import { estimateHours } from '@/entities/item'
import type { Estimate } from '@/shared/api'
import { reasonLabel, type UndatedColumn } from '../model/columns'

function FoldedTab({ column, onUnfold }: { column: UndatedColumn; onUnfold: () => void }) {
  return (
    <button
      type="button"
      aria-expanded={false}
      aria-label={`${column.name}, ${column.entries.length}`}
      title={column.parking ? `Parked in ${column.name}: not scheduled` : 'Not scheduled'}
      onClick={onUnfold}
      className="flex w-9 shrink-0 flex-col items-center gap-2 rounded-lg border border-border bg-muted/40 py-3 text-xs text-muted-foreground transition-colors duration-(--duration-hover) ease-(--ease-hover) hover:bg-muted hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-ring"
    >
      <span className="rounded-full bg-background px-1.5 font-mono text-[11px] text-foreground">{column.entries.length}</span>
      <span className="font-medium [writing-mode:vertical-rl]">{column.name}</span>
    </button>
  )
}

function UnfoldedList({
  column,
  estimates,
  onFold,
  onOpen,
}: {
  column: UndatedColumn
  estimates: ReadonlyMap<string, Estimate>
  onFold: () => void
  onOpen: (reference: string) => void
}) {
  return (
    <section aria-label={column.name} className="flex w-72 shrink-0 flex-col overflow-hidden rounded-lg border border-border">
      <header className="flex h-[33px] shrink-0 items-center gap-2 border-b border-border bg-muted/40 px-3">
        <span className="truncate text-sm font-medium">{column.name}</span>
        <span className="font-mono text-[11px] text-muted-foreground">{column.entries.length}</span>
        <span className="flex-1" />
        <button
          type="button"
          aria-expanded
          aria-label={`Fold ${column.name}`}
          title="Fold"
          onClick={onFold}
          className="icon-button size-6"
        >
          <ChevronsRight className="size-4" aria-hidden />
        </button>
      </header>
      <ul className="min-h-0 flex-1 overflow-y-auto py-1">
        {column.entries.map((entry) => {
          const estimate = estimates.get(entry.reference)
          return (
            <li key={entry.reference}>
              <button
                type="button"
                onClick={() => onOpen(entry.reference)}
                className="flex w-full flex-col gap-0.5 px-3 py-1.5 text-left hover:bg-muted/60 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring"
              >
                <span className="flex min-w-0 items-center gap-2 text-sm">
                  <span className="shrink-0 font-mono text-[11px] text-muted-foreground">{entry.id}</span>
                  <span className="truncate">{entry.title}</span>
                  {estimate && (
                    <span className="ml-auto shrink-0 font-mono text-[11px] text-muted-foreground">{estimateHours(estimate)}</span>
                  )}
                </span>
                {!column.parking && <span className="text-[11px] text-muted-foreground">{reasonLabel(entry)}</span>}
              </button>
            </li>
          )
        })}
      </ul>
    </section>
  )
}

export function UndatedColumns({
  columns,
  unfolded,
  estimates,
  onToggle,
  onOpen,
}: {
  columns: UndatedColumn[]
  unfolded: readonly string[]
  estimates: ReadonlyMap<string, Estimate>
  onToggle: (key: string) => void
  onOpen: (reference: string) => void
}) {
  if (columns.length === 0) {
    return null
  }
  return (
    <div className="flex shrink-0 gap-2" aria-label="Not scheduled">
      {columns.map((column) =>
        unfolded.includes(column.key) ? (
          <UnfoldedList key={column.key} column={column} estimates={estimates} onFold={() => onToggle(column.key)} onOpen={onOpen} />
        ) : (
          <FoldedTab key={column.key} column={column} onUnfold={() => onToggle(column.key)} />
        ),
      )}
    </div>
  )
}
