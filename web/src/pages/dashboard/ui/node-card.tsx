import { ArrowUpRight } from 'lucide-react'
import type { ReactNode } from 'react'
import { Link } from 'react-router'
import type { Category, Item } from '@/shared/api'
import { cn, displayNode } from '@/shared/lib'
import type { CardStats } from '../model/card-stats'
import { CategoryBar } from './category-bar'

function Section({ title, items, empty }: { title: string; items: Item[]; empty: string }) {
  return (
    <div>
      <h3 className="mb-1 text-xs font-semibold tracking-wide text-muted-foreground uppercase">{title}</h3>
      {items.length === 0 ? (
        <p className="text-xs text-muted-foreground">{empty}</p>
      ) : (
        <ul className="space-y-0.5 text-sm">
          {items.map((item) => (
            <li key={item.path} className="flex gap-2">
              <span className="shrink-0 font-mono text-xs text-muted-foreground">{item.id ?? '—'}</span>
              <span className="truncate" title={item.title ?? item.path}>
                {item.title ?? item.path}
              </span>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}

export function NodeCard({
  path,
  stats,
  labels,
  toolbar,
  className,
}: {
  path: string
  stats: CardStats
  labels: (category: Category) => string
  toolbar?: ReactNode
  className?: string
}) {
  return (
    <article className={cn('flex flex-col gap-3 rounded-lg border border-border bg-background p-4', className)}>
      <header className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <Link
            to={{ pathname: '/tasks', search: `?node=${encodeURIComponent(path)}` }}
            className="inline-flex items-center gap-1 font-mono text-sm font-medium hover:underline"
          >
            <span className="truncate">{displayNode(path)}</span>
            <ArrowUpRight className="size-3.5 shrink-0" aria-hidden />
          </Link>
          <p className="text-xs text-muted-foreground">
            {stats.total} live items · last movement {stats.lastChange ?? 'unknown'}
          </p>
        </div>
        {toolbar}
      </header>
      <CategoryBar counts={stats.counts} total={stats.total} labels={labels} />
      <div className="grid grid-cols-2 gap-3">
        <Section title="Quick wins" items={stats.quickWins} empty="None — size XS/S × impact high" />
        <Section title="Blockers" items={stats.blockers} empty="Nothing blocked" />
      </div>
    </article>
  )
}
