import type { ReactNode } from 'react'
import { excerpt, ItemLink } from '@/entities/item'
import type { Item } from '@/shared/api'
import { cn, displayNode } from '@/shared/lib'
import { Highlight } from '@/shared/ui'

export function ReflexionCard({ item, scope, action, className }: { item: Item; scope: string | null; action?: ReactNode; className?: string }) {
  const summary = excerpt(item.body)
  return (
    <article className={cn('space-y-1.5 rounded-md border border-border bg-background p-3 text-sm shadow-xs', className)}>
      <h3 className="leading-snug font-medium">
        <ItemLink item={item}>
          <Highlight text={item.title ?? item.path} />
        </ItemLink>
      </h3>
      {summary && (
        <p className="line-clamp-3 text-xs leading-relaxed text-muted-foreground">
          <Highlight text={summary} />
        </p>
      )}
      <div className="flex flex-wrap items-center gap-x-2 gap-y-1 font-mono text-[10px] text-muted-foreground">
        <span>
          <Highlight text={item.id ?? ''} />
        </span>
        {item.node !== scope && <span className="truncate">{displayNode(item.node)}</span>}
        {item.archived && <span>archived</span>}
        {(item.updated ?? item.created) && <span className="ml-auto">{item.updated ?? item.created}</span>}
      </div>
      {action}
    </article>
  )
}
