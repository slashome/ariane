import { CircleAlert } from 'lucide-react'
import { isClosed, ItemLink, StatusBadge } from '@/entities/item'
import { statusLabel, useNodes } from '@/entities/node'
import type { Item } from '@/shared/api'
import { cn, displayNode } from '@/shared/lib'

export function RelatedList({ title, items, scope, empty }: { title: string; items: Item[]; scope: string; empty?: string }) {
  const { data: nodes = [] } = useNodes()
  if (items.length === 0 && empty === undefined) {
    return null
  }
  return (
    <section className="space-y-1.5">
      <h3 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
        {title} <span className="font-normal">({items.length})</span>
      </h3>
      {items.length === 0 ? (
        <p className="text-sm text-muted-foreground">{empty}</p>
      ) : (
        <ul className="space-y-1">
          {items.map((item) => (
            <li key={item.path} className="flex items-start gap-2 text-sm">
              <span className="w-14 shrink-0 pt-0.5 font-mono text-xs text-muted-foreground">{item.id ?? '—'}</span>
              <span className={cn('min-w-0 flex-1', isClosed(item.category) && 'text-muted-foreground')}>
                <ItemLink item={item}>{item.title ?? item.path}</ItemLink>
                {item.node !== scope && (
                  <span className="ml-1 font-mono text-[10px] text-muted-foreground">{displayNode(item.node)}</span>
                )}
              </span>
              <StatusBadge status={item.status} category={item.category} label={statusLabel(nodes, item.node, item.status)} />
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}

export function Unresolved({ references }: { references: string[] }) {
  if (references.length === 0) {
    return null
  }
  return (
    <p className="flex items-start gap-1.5 text-xs text-red-700 dark:text-red-300">
      <CircleAlert className="mt-px size-3.5 shrink-0" aria-hidden />
      Points to no item: {references.join(', ')}
    </p>
  )
}
