import type { ReactNode } from 'react'
import { excerpt, ItemLink } from '@/entities/item'
import type { Item } from '@/shared/api'
import { cn, displayNode } from '@/shared/lib'
import { Highlight } from '@/shared/ui'
import './reflexions.css'

export function ReflexionCard({ item, scope, action, className }: { item: Item; scope: string | null; action?: ReactNode; className?: string }) {
  const summary = excerpt(item.body)
  return (
    <article className={cn('reflexion-card', className)}>
      <h3 className="reflexion-card__title">
        <ItemLink item={item}>
          <Highlight text={item.title ?? item.path} />
        </ItemLink>
      </h3>
      {summary && (
        <p className="lu-line-clamp reflexion-card__summary">
          <Highlight text={summary} />
        </p>
      )}
      <div className="reflexion-card__meta">
        <span>
          <Highlight text={item.id ?? ''} />
        </span>
        {item.node !== scope && <span className="lu-truncate">{displayNode(item.node)}</span>}
        {item.archived && <span>archived</span>}
        {(item.updated ?? item.created) && <span className="reflexion-card__date">{item.updated ?? item.created}</span>}
      </div>
      {action}
    </article>
  )
}
