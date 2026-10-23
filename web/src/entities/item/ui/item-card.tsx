import { Hourglass } from 'lucide-react'
import type { HTMLAttributes, ReactNode } from 'react'
import type { Item } from '@/shared/api'
import { cn, displayNode } from '@/shared/lib'
import { Highlight } from '@/shared/ui'

function Chip({ children }: { children: ReactNode }) {
  return <span className="rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">{children}</span>
}

export function ItemCard({
  item,
  showNode,
  aside,
  action,
  waiting,
  selected,
  className,
  ...props
}: {
  item: Item
  showNode?: boolean
  aside?: ReactNode
  action?: ReactNode
  // Who the item waits for and since when, already put into words.
  waiting?: string
  // The card shown in the preview panel: outlined in the accent color.
  selected?: boolean
} & HTMLAttributes<HTMLDivElement>) {
  const chips = [
    item.size && `size ${item.size}`,
    item.impact && `impact ${item.impact}`,
    item.priority && `prio ${item.priority}`,
  ].filter((chip): chip is string => Boolean(chip))
  return (
    <div
      className={cn(
        'rounded-md border border-border bg-background p-2.5 text-sm shadow-xs',
        selected && 'border-accent ring-2 ring-accent/40',
        className,
      )}
      {...props}
    >
      <div className="flex items-start justify-between gap-2">
        <p className="line-clamp-3 min-w-0 leading-snug">
          <Highlight text={item.title ?? item.path} />
        </p>
        {aside}
      </div>
      {waiting && (
        <p className="mt-1.5 inline-flex items-center gap-1 rounded bg-violet-500/15 px-1.5 py-0.5 text-[11px] text-violet-700 dark:text-violet-300">
          <Hourglass className="size-3" aria-hidden />
          {waiting}
        </p>
      )}
      <div className="mt-2 flex flex-wrap items-center gap-1">
        <span className="font-mono text-[10px] text-muted-foreground">
          <Highlight text={item.id ?? '—'} />
        </span>
        {showNode && <span className="truncate font-mono text-[10px] text-muted-foreground">{displayNode(item.node)}</span>}
        {chips.map((chip) => (
          <Chip key={chip}>{chip}</Chip>
        ))}
        {item.external.map((link) => (
          <Chip key={link}>{link}</Chip>
        ))}
        {action && <span className="ml-auto">{action}</span>}
      </div>
    </div>
  )
}
