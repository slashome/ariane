import { CalendarOff } from 'lucide-react'
import { Link, useLocation } from 'react-router'
import { routes, scopeSearch } from '@/shared/config'

export function CalendarBanner() {
  const { search } = useLocation()
  return (
    <div role="note" aria-label="No calendar" className="flex flex-wrap items-center gap-x-2 gap-y-1 rounded-lg border border-border bg-muted/40 px-3 py-1.5 text-xs text-muted-foreground">
      <CalendarOff className="size-4 shrink-0" aria-hidden />
      <span>No work calendar covers this node, so the bars are relative focused hours from now, not dates.</span>
      <span>
        Add one in{' '}
        <Link to={{ pathname: routes.roadmapSettings, search: scopeSearch(search) }} className="font-medium text-foreground underline underline-offset-2 hover:text-primary">
          Roadmap › Settings
        </Link>{' '}
        or with <code className="rounded bg-muted px-1 font-mono text-[11px] text-foreground">ariane calendar edit</code>.
      </span>
    </div>
  )
}
