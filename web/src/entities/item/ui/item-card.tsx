import type { HTMLAttributes, ReactNode } from 'react'
import type { Item } from '@/shared/api'
import { cn, displayNode } from '@/shared/lib'

function Chip({ children }: { children: ReactNode }) {
  return <span className="rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">{children}</span>
}

export function ItemCard({
  item,
  showNode,
  aside,
  className,
  ...props
}: { item: Item; showNode?: boolean; aside?: ReactNode } & HTMLAttributes<HTMLDivElement>) {
  const chips = [
    item.size && `size ${item.size}`,
    item.impact && `impact ${item.impact}`,
    item.priority && `prio ${item.priority}`,
  ].filter((chip): chip is string => Boolean(chip))
  return (
    <div
      className={cn('rounded-md border border-border bg-background p-2.5 text-sm shadow-xs', className)}
      {...props}
    >
      <div className="flex items-start justify-between gap-2">
        <p className="line-clamp-3 min-w-0 leading-snug">{item.title ?? item.path}</p>
        {aside}
      </div>
      <div className="mt-2 flex flex-wrap items-center gap-1">
        <span className="font-mono text-[10px] text-muted-foreground">{item.id ?? '—'}</span>
        {showNode && <span className="truncate font-mono text-[10px] text-muted-foreground">{displayNode(item.node)}</span>}
        {chips.map((chip) => (
          <Chip key={chip}>{chip}</Chip>
        ))}
      </div>
    </div>
  )
}
