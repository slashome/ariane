import { ChevronDown } from 'lucide-react'
import { useId, useState, type ReactNode } from 'react'
import { cn } from '@/shared/lib'

type Override = { when: boolean; open: boolean }

export function Fold({
  title,
  summary,
  openWhen,
  children,
}: {
  title: string
  summary: ReactNode
  openWhen: boolean
  children: ReactNode
}) {
  const [override, setOverride] = useState<Override | null>(null)
  const open = override !== null && override.when === openWhen ? override.open : openWhen
  const body = useId()
  return (
    <section className="space-y-2" aria-label={title}>
      <div className="flex flex-wrap items-center justify-between gap-2">
        <button
          type="button"
          aria-expanded={open}
          aria-controls={body}
          onClick={() => setOverride({ when: openWhen, open: !open })}
          className="inline-flex items-center gap-1.5 text-sm font-semibold hover:text-foreground"
        >
          <ChevronDown className={cn('size-4 shrink-0 transition-transform', !open && '-rotate-90')} aria-hidden />
          {title}
        </button>
        {summary}
      </div>
      {open && (
        <div id={body} className="space-y-2">
          {children}
        </div>
      )}
    </section>
  )
}
