import type { ReactNode } from 'react'
import { Link } from 'react-router'
import type { Category, Item } from '@/shared/api'
import { cn, displayNode } from '@/shared/lib'
import { Icon, Markdown } from '@/shared/ui'
import type { CardStats } from '../model/card-stats'
import { CategoryBar } from '@/entities/item'
import './dashboard.css'

function Section({ title, items, empty }: { title: string; items: Item[]; empty: string }) {
  return (
    <div>
      <h3 className="node-card__section-title">{title}</h3>
      {items.length === 0 ? (
        <p className="node-card__meta">{empty}</p>
      ) : (
        <ul className="node-card__list">
          {items.map((item) => (
            <li key={item.path} className="node-card__entry">
              <span className="node-card__entry-id">{item.id ?? '—'}</span>
              <span className="lu-truncate" title={item.title ?? item.path}>
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
  description,
  stats,
  labels,
  toolbar,
  className,
}: {
  path: string
  description?: string | null
  stats: CardStats
  labels: (category: Category) => string
  toolbar?: ReactNode
  className?: string
}) {
  return (
    <article className={cn('node-card', className)}>
      <header className="node-card__header">
        <div className="node-card__heading">
          <Link
            to={{ pathname: '/tasks', search: `?node=${encodeURIComponent(path)}` }}
            className="node-card__link"
          >
            <span className="lu-truncate">{displayNode(path)}</span>
            <Icon name="arrow-up-right" className="node-card__link-icon" />
          </Link>
          <p className="node-card__meta">
            {stats.total} live items · last movement {stats.lastChange ?? 'unknown'}
          </p>
        </div>
        {toolbar}
      </header>
      {description && (
        <details className="node-card__about">
          <summary className="node-card__section-title node-card__summary">About</summary>
          <Markdown source={description} className="node-card__description" />
        </details>
      )}
      <CategoryBar counts={stats.counts} labels={labels} />
      <div className="node-card__sections">
        <Section title="Quick wins" items={stats.quickWins} empty="None — size XS/S × impact high" />
        <Section title="Blockers" items={stats.blockers} empty="Nothing blocked" />
      </div>
    </article>
  )
}
