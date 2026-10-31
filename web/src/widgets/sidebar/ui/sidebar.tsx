import { useMemo, useState, type ReactNode } from 'react'
import { NavLink, useLocation } from 'react-router'
import { scopeFindings, useDoctor } from '@/entities/doctor'
import { isClosed, useItems } from '@/entities/item'
import { useDashboard } from '@/entities/node'
import { useProxy } from '@/entities/proxy'
import { useRepo } from '@/entities/repo'
import { inScope, scopeItems, useItemFilters } from '@/features/filter-items'
import { routes, scopeSearch, searchParamKeys } from '@/shared/config'
import { cn, displayNode } from '@/shared/lib'
import { Brand, Icon } from '@/shared/ui'
import { navCounts } from '../model/nav-counts'
import { proxyEntry } from '../model/proxy-entry'
import './sidebar.css'

const proxyRefetch = 30_000

const collapsedKey = 'ariane.sidebar.collapsed'

function readCollapsed(): Set<string> {
  try {
    return new Set(JSON.parse(localStorage.getItem(collapsedKey) ?? '[]') as string[])
  } catch {
    return new Set()
  }
}

function Count({ value, tone }: { value: number | undefined; tone?: string }) {
  if (value === undefined) {
    return null
  }
  return <span className={cn('sidebar-count', tone)}>{value}</span>
}

function Entry({
  to,
  label,
  icon,
  count,
  tone,
  dot,
  status,
  end,
  search,
  nested,
}: {
  to: string
  label: string
  icon: string
  count?: number
  tone?: string
  dot?: string
  status?: string
  end?: boolean
  search: string
  nested?: boolean
}) {
  const named = [label, count, status].filter((part) => part !== undefined).join(', ')
  return (
    <NavLink
      to={{ pathname: to, search }}
      end={end}
      aria-label={named}
      className={({ isActive }) =>
        cn(
          'sidebar-entry',
          nested && 'sidebar-entry--nested',
          isActive && 'sidebar-entry--active',
        )
      }
    >
      <Icon name={icon} className="sidebar-entry__icon" />
      <span className="lu-truncate">{label}</span>
      {dot && <span data-testid="status-dot" className={cn('sidebar-entry__dot', `sidebar-entry__dot--${dot}`)} aria-hidden />}
      <Count value={count} tone={tone} />
    </NavLink>
  )
}

function Section({
  id,
  label,
  icon,
  collapsed,
  onToggle,
  children,
}: {
  id: string
  label: string
  icon: string
  collapsed: boolean
  onToggle: () => void
  children: ReactNode
}) {
  return (
    <div>
      <button
        type="button"
        aria-expanded={!collapsed}
        aria-controls={`nav-${id}`}
        onClick={onToggle}
        className="sidebar-section__toggle"
      >
        <Icon name={icon} className="sidebar-entry__icon" />
        {label}
        <Icon name="chevron-down" className={cn('sidebar-section__chevron', collapsed && 'sidebar-section__chevron--collapsed')} />
      </button>
      <div id={`nav-${id}`} hidden={collapsed} className="sidebar-section__entries">
        {children}
      </div>
    </div>
  )
}

export function Sidebar() {
  const { search, pathname } = useLocation()
  const { filters } = useItemFilters()
  const { data: items = [] } = useItems()
  const { data: findings = [] } = useDoctor()
  const { data: repo } = useRepo()
  const { data: dashboard } = useDashboard()
  const { data: proxyReport } = useProxy({ refetchInterval: proxyRefetch })
  const proxy = proxyEntry(proxyReport)
  const [collapsed, setCollapsed] = useState(readCollapsed)
  const scope = scopeSearch(search)

  const counts = useMemo(
    () => navCounts(scopeItems(items, filters), scopeFindings(findings, filters.node, filters.includeDescendants)),
    [items, findings, filters],
  )
  const pins = useMemo(
    () =>
      (dashboard?.pinned ?? []).map((path) => ({
        path,
        open: items.filter(
          (item) =>
            !item.archived &&
            inScope(item.node, path, filters.includeDescendants) &&
            item.category !== null &&
            !isClosed(item.category),
        ).length,
      })),
    [dashboard, items, filters.includeDescendants],
  )

  const toggle = (id: string) =>
    setCollapsed((current) => {
      const next = new Set(current)
      if (next.has(id)) {
        next.delete(id)
      } else {
        next.add(id)
      }
      try {
        localStorage.setItem(collapsedKey, JSON.stringify([...next]))
      } catch {
        // A browser that refuses storage only forgets the folding.
      }
      return next
    })

  const pinSearch = (path: string) => {
    const next = new URLSearchParams(scope)
    next.set(searchParamKeys.node, path)
    return `?${next.toString()}`
  }

  return (
    <aside className="sidebar">
      <div className="sidebar__brand">
        <Brand />
      </div>
      <nav aria-label="Main" className="sidebar__nav sidebar__nav--main">
        <Section id="home" label="Home" icon="home" collapsed={collapsed.has('home')} onToggle={() => toggle('home')}>
          <Entry to={routes.home} end label="Dashboard" icon="dashboard" search={scope} nested />
          {pins.map((pin) => (
            <NavLink
              key={pin.path}
              to={{ pathname: pathname, search: pinSearch(pin.path) }}
              aria-label={`Scope to ${displayNode(pin.path)}, ${pin.open} open items`}
              aria-current={filters.node === pin.path ? 'true' : undefined}
              className={cn(
                'sidebar-entry sidebar-entry--nested',
                filters.node === pin.path && 'sidebar-entry--pinned',
              )}
              title={`Scope every view to ${displayNode(pin.path)}`}
            >
              <Icon name="pin" className="sidebar-entry__pin" />
              <span className="sidebar-entry__path lu-truncate">{displayNode(pin.path)}</span>
              <Count value={pin.open} />
            </NavLink>
          ))}
        </Section>
        <Section id="tasks" label="Tasks" icon="todo" collapsed={collapsed.has('tasks')} onToggle={() => toggle('tasks')}>
          <Entry to={routes.taskList} label="List" icon="list" count={counts.tasks} search={scope} nested />
          <Entry to={routes.kanban} label="Kanban" icon="columns" count={counts.open} search={scope} nested />
          <Entry to={routes.matrix} label="Size × impact" icon="squares" count={counts.toEstimate} search={scope} nested />
          <Entry to={routes.backlog} label="Backlog" icon="inbox" count={counts.backlog} search={scope} nested />
        </Section>
        <Entry to={routes.epics} label="Epics" icon="layers" count={counts.epics} search={scope} />
        <Entry to={routes.stories} label="Stories" icon="book" count={counts.stories} search={scope} />
        <Section id="reflexions" label="Reflexions" icon="idea" collapsed={collapsed.has('reflexions')} onToggle={() => toggle('reflexions')}>
          <Entry to={routes.reflexions} end label="Board" icon="columns" count={counts.reflexions} search={scope} nested />
          <Entry to={routes.journal} label="Journal" icon="notebook" count={counts.settled} search={scope} nested />
        </Section>
        <Entry to={routes.ressources} label="Ressources" icon="library" count={counts.ressources} search={scope} />
      </nav>
      <nav aria-label="System" className="sidebar__nav sidebar__nav--system">
        <Entry
          to={routes.health}
          label="Health"
          icon="health"
          count={counts.errors}
          tone={counts.errors > 0 ? 'sidebar-count--danger' : undefined}
          search={scope}
        />
        <Entry
          to={routes.repo}
          label="Repository"
          icon="repository"
          count={repo?.uncommitted.length}
          tone={(repo?.uncommitted.length ?? 0) > 0 ? 'sidebar-count--warning' : undefined}
          search={scope}
        />
        <Entry to={routes.proxy} label="Proxy" icon="network" count={proxy.count} dot={proxy.state} status={proxy.status} search={scope} />
        <Entry to={routes.people} label="People" icon="users" search={scope} />
        <Entry to={routes.settings} label="Settings" icon="settings" search={scope} />
      </nav>
    </aside>
  )
}
