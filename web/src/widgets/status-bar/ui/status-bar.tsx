import type { ReactNode } from 'react'
import { Link, useLocation } from 'react-router'
import { countBySeverity, scopeFindings, useDoctor } from '@/entities/doctor'
import { useItems } from '@/entities/item'
import { useRepo } from '@/entities/repo'
import { ArchivedToggle, scopeItems, useItemFilters } from '@/features/filter-items'
import { routes, scopeSearch } from '@/shared/config'
import { cn, displayNode } from '@/shared/lib'
import { Icon } from '@/shared/ui'
import './status-bar.css'

function Cell({ children, to, label, className }: { children: ReactNode; to?: string; label: string; className?: string }) {
  const { search } = useLocation()
  const classes = cn('status-bar__cell', className)
  return to ? (
    <Link to={{ pathname: to, search: scopeSearch(search) }} aria-label={label} title={label} className={cn(classes, 'status-bar__cell--link')}>
      {children}
    </Link>
  ) : (
    <span aria-label={label} title={label} className={classes}>
      {children}
    </span>
  )
}

// The bottom line of every screen: where the content stands in git, which node every view is scoped
// to, what doctor thinks of it, and the toggle that brings archived items into every view.
export function StatusBar() {
  const { data: repo } = useRepo()
  const { data: findings = [] } = useDoctor()
  const { data: items = [] } = useItems()
  const { filters } = useItemFilters()
  const { errors, warnings } = countBySeverity(scopeFindings(findings, filters.node, filters.includeDescendants))
  const visible = scopeItems(items, filters).length
  const uncommitted = repo?.uncommitted.length ?? 0
  return (
    <footer
      aria-label="Status"
      className="status-bar"
    >
      <Cell to={routes.repo} label={`Branch ${repo?.branch ?? 'unknown'}`} className="status-bar__cell--mono">
        <Icon name="branch" className="status-bar__icon" />
        {repo?.branch ?? '…'}
      </Cell>
      {(repo?.ahead ?? 0) > 0 && (
        <Cell to={routes.repo} label={`${repo?.ahead} commits to push`}>
          <Icon name="upload" className="status-bar__icon" />
          {repo?.ahead}
        </Cell>
      )}
      <Cell
        to={routes.repo}
        label={`${uncommitted} uncommitted ${uncommitted === 1 ? 'file' : 'files'}`}
        className={uncommitted > 0 ? 'status-bar__cell--warning' : undefined}
      >
        <Icon name="commit" className="status-bar__icon" />
        {uncommitted} uncommitted
      </Cell>
      <Cell label="Node every view is scoped to">
        <Icon name="network" className="status-bar__icon" />
        <span className="status-bar__mono">{filters.node === null ? 'all nodes' : displayNode(filters.node)}</span>
        {filters.node !== null && filters.includeDescendants && <span>+ sub-nodes</span>}
      </Cell>
      <Cell label={`${visible} items in scope`}>{visible} items</Cell>
      <Cell
        to={routes.health}
        label={`${errors} errors, ${warnings} warnings from doctor`}
        className={errors > 0 ? 'status-bar__cell--danger' : undefined}
      >
        {errors > 0 ? <Icon name="alert" className="status-bar__icon" /> : <Icon name="warning" className="status-bar__icon" />}
        {errors === 0 && warnings === 0 ? 'healthy' : `${errors} errors · ${warnings} warnings`}
      </Cell>
      <div className="status-bar__end">
        <ArchivedToggle />
      </div>
    </footer>
  )
}
