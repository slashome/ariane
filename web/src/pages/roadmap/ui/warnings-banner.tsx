import { TriangleAlert, X } from 'lucide-react'
import type { ForecastWarningKind } from '@/shared/api'
import { cn } from '@/shared/lib'
import { Tooltip } from '@/shared/ui'
import type { WarningCounter } from '../model/warnings'

export function WarningsBanner({
  counters,
  active,
  onToggle,
}: {
  counters: WarningCounter[]
  active: ForecastWarningKind | null
  onToggle: (kind: ForecastWarningKind) => void
}) {
  if (counters.length === 0) {
    return null
  }
  return (
    <div
      role="group"
      aria-label="Warnings"
      className="flex flex-wrap items-center gap-1.5 rounded-lg border border-amber-500/30 bg-amber-500/5 px-3 py-1.5"
    >
      <TriangleAlert className="size-4 shrink-0 text-amber-600 dark:text-amber-400" aria-hidden />
      <span className="mr-1 text-xs text-muted-foreground">Warnings</span>
      {counters.map((counter) => {
        const pressed = counter.kind === active
        return (
          <Tooltip key={counter.kind} content={counter.hint}>
            <button
              type="button"
              aria-pressed={pressed}
              aria-label={`${counter.label}, ${counter.count}`}
              onClick={() => onToggle(counter.kind)}
              className={cn(
                'inline-flex items-center gap-1.5 rounded-full border border-border bg-background px-2.5 py-0.5 text-xs transition-colors duration-(--duration-hover) ease-(--ease-hover) hover:bg-muted focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-ring',
                pressed && 'border-primary bg-primary text-primary-foreground hover:bg-primary',
              )}
            >
              {counter.label}
              <span className={cn('font-mono text-[11px] text-muted-foreground', pressed && 'text-primary-foreground')}>{counter.count}</span>
              {pressed && <X className="size-3" aria-hidden />}
            </button>
          </Tooltip>
        )
      })}
    </div>
  )
}
