import { CircleAlert, GitBranch, GitCommitHorizontal, Network, TriangleAlert, Upload } from 'lucide-react'
import type { ReactNode } from 'react'
import { Link, useLocation } from 'react-router'
import { countBySeverity, scopeFindings, useDoctor } from '@/entities/doctor'
import { useItems } from '@/entities/item'
import { useRepo } from '@/entities/repo'
import { ArchivedToggle, scopeItems, useItemFilters } from '@/features/filter-items'
import { routes, scopeSearch } from '@/shared/config'
import { cn, displayNode } from '@/shared/lib'

function Cell({ children, to, label, className }: { children: ReactNode; to?: string; label: string; className?: string }) {
  const { search } = useLocation()
  const classes = cn('flex h-full items-center gap-1 px-2', className)
  return to ? (
    <Link to={{ pathname: to, search: scopeSearch(search) }} aria-label={label} title={label} className={cn(classes, 'hover:bg-muted hover:text-foreground')}>
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
      className="sticky bottom-0 z-20 flex h-7 shrink-0 items-center border-t border-border bg-sidebar text-xs text-muted-foreground"
    >
      <Cell to={routes.repo} label={`Branch ${repo?.branch ?? 'unknown'}`} className="font-mono">
        <GitBranch className="size-3.5" aria-hidden />
        {repo?.branch ?? '…'}
      </Cell>
      {(repo?.ahead ?? 0) > 0 && (
        <Cell to={routes.repo} label={`${repo?.ahead} commits to push`}>
          <Upload className="size-3.5" aria-hidden />
          {repo?.ahead}
        </Cell>
      )}
      <Cell
        to={routes.repo}
        label={`${uncommitted} uncommitted ${uncommitted === 1 ? 'file' : 'files'}`}
        className={uncommitted > 0 ? 'text-amber-700 dark:text-amber-300' : undefined}
      >
        <GitCommitHorizontal className="size-3.5" aria-hidden />
        {uncommitted} uncommitted
      </Cell>
      <Cell label="Node every view is scoped to">
        <Network className="size-3.5" aria-hidden />
        <span className="font-mono">{filters.node === null ? 'all nodes' : displayNode(filters.node)}</span>
        {filters.node !== null && filters.includeDescendants && <span>+ sub-nodes</span>}
      </Cell>
      <Cell label={`${visible} items in scope`}>{visible} items</Cell>
      <Cell
        to={routes.health}
        label={`${errors} errors, ${warnings} warnings from doctor`}
        className={errors > 0 ? 'text-red-700 dark:text-red-300' : undefined}
      >
        {errors > 0 ? <CircleAlert className="size-3.5" aria-hidden /> : <TriangleAlert className="size-3.5" aria-hidden />}
        {errors === 0 && warnings === 0 ? 'healthy' : `${errors} errors · ${warnings} warnings`}
      </Cell>
      <div className="ml-auto flex h-full items-center px-2">
        <ArchivedToggle />
      </div>
    </footer>
  )
}
