import type { HTMLAttributes, ReactNode } from 'react'
import type { Item } from '@/shared/api'
import { cn, displayNode } from '@/shared/lib'
import { Highlight, Icon } from '@/shared/ui'
import './item.css'

function Chip({ children }: { children: ReactNode }) {
  return <span className="item-card__chip">{children}</span>
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
        'item-card',
        selected && 'item-card--selected',
        className,
      )}
      {...props}
    >
      <div className="item-card__head">
        <p className="lu-line-clamp item-card__title">
          <Highlight text={item.title ?? item.path} />
        </p>
        {aside}
      </div>
      {waiting && (
        <p className="item-card__waiting">
          <Icon name="waiting" className="item-card__waiting-icon" />
          {waiting}
        </p>
      )}
      <div className="item-card__meta">
        <span className="item-card__id">
          <Highlight text={item.id ?? '—'} />
        </span>
        {showNode && <span className="lu-truncate item-card__node">{displayNode(item.node)}</span>}
        {chips.map((chip) => (
          <Chip key={chip}>{chip}</Chip>
        ))}
        {item.external.map((link) => (
          <Chip key={link}>{link}</Chip>
        ))}
        {action && <span className="item-card__action">{action}</span>}
      </div>
    </div>
  )
}
