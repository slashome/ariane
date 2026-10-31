import { isClosed, ItemLink, StatusBadge } from '@/entities/item'
import { statusLabel, useNodes } from '@/entities/node'
import type { Item } from '@/shared/api'
import { cn, displayNode } from '@/shared/lib'
import { Icon } from '@/shared/ui'
import './item-view.css'

export function RelatedList({ title, items, scope, empty }: { title: string; items: Item[]; scope: string; empty?: string }) {
  const { data: nodes = [] } = useNodes()
  if (items.length === 0 && empty === undefined) {
    return null
  }
  return (
    <section className="item-view__panel">
      <h3 className="item-view__panel-title">
        {title} <span className="item-view__panel-count">({items.length})</span>
      </h3>
      {items.length === 0 ? (
        <p className="related-list__empty">{empty}</p>
      ) : (
        <ul className="item-view__list">
          {items.map((item) => (
            <li key={item.path} className="related-list__item">
              <span className="related-list__id">{item.id ?? '—'}</span>
              <span className={cn('related-list__title', isClosed(item.category) && 'related-list__title--closed')}>
                <ItemLink item={item}>{item.title ?? item.path}</ItemLink>
                {item.node !== scope && (
                  <span className="related-list__node">{displayNode(item.node)}</span>
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
    <p className="related-list__unresolved">
      <Icon name="alert" className="related-list__unresolved-icon" />
      Points to no item: {references.join(', ')}
    </p>
  )
}
