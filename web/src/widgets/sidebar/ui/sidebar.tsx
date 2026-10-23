import {
  BookOpen,
  ChevronDown,
  Columns3,
  FolderGit2,
  Grid2x2,
  HeartPulse,
  Home,
  Inbox,
  LayoutDashboard,
  Library,
  Lightbulb,
  List,
  ListTodo,
  NotebookPen,
  Pin,
  Settings,
  Users,
  type LucideIcon,
} from 'lucide-react'
import { useMemo, useState, type ReactNode } from 'react'
import { NavLink, useLocation } from 'react-router'
import { scopeFindings, useDoctor } from '@/entities/doctor'
import { isClosed, useItems } from '@/entities/item'
import { useDashboard } from '@/entities/node'
import { useRepo } from '@/entities/repo'
import { inScope, scopeItems, useItemFilters } from '@/features/filter-items'
import { routes, scopeSearch, searchParamKeys } from '@/shared/config'
import { cn, displayNode } from '@/shared/lib'
import { Brand } from '@/shared/ui'
import { navCounts } from '../model/nav-counts'

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
  return <span className={cn('ml-auto min-w-5 rounded-full px-1.5 text-center text-[11px] tabular-nums text-muted-foreground', tone)}>{value}</span>
}

function Entry({
  to,
  label,
  icon: Icon,
  count,
  tone,
  end,
  search,
  nested,
}: {
  to: string
  label: string
  icon: LucideIcon
  count?: number
  tone?: string
  end?: boolean
  search: string
  nested?: boolean
}) {
  return (
    <NavLink
      to={{ pathname: to, search }}
      end={end}
      aria-label={count === undefined ? label : `${label}, ${count}`}
      className={({ isActive }) =>
        cn(
          'flex items-center gap-2.5 rounded-md py-1.5 pr-2 text-sm text-muted-foreground hover:bg-muted hover:text-foreground',
          nested ? 'pl-8' : 'pl-3',
          isActive && 'bg-accent/15 font-medium text-foreground shadow-[inset_2px_0_0_var(--color-accent)]',
        )
      }
    >
      <Icon className="size-4 shrink-0" aria-hidden />
      <span className="truncate">{label}</span>
      <Count value={count} tone={tone} />
    </NavLink>
  )
}

function Section({
  id,
  label,
  icon: Icon,
  collapsed,
  onToggle,
  children,
}: {
  id: string
  label: string
  icon: LucideIcon
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
        className="flex w-full items-center gap-2.5 rounded-md px-3 py-1.5 text-sm font-medium text-foreground hover:bg-muted"
      >
        <Icon className="size-4 shrink-0" aria-hidden />
        {label}
        <ChevronDown className={cn('ml-auto size-3.5 text-muted-foreground transition-transform', collapsed && '-rotate-90')} aria-hidden />
      </button>
      <div id={`nav-${id}`} hidden={collapsed} className="mt-0.5 flex flex-col gap-0.5">
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
    <aside className="flex w-60 shrink-0 flex-col border-r border-border bg-sidebar">
      <div className="flex h-14 shrink-0 items-center gap-2 border-b border-border px-4 font-semibold">
        <Brand />
      </div>
      <nav aria-label="Main" className="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto p-2">
        <Section id="home" label="Home" icon={Home} collapsed={collapsed.has('home')} onToggle={() => toggle('home')}>
          <Entry to={routes.home} end label="Dashboard" icon={LayoutDashboard} search={scope} nested />
          {pins.map((pin) => (
            <NavLink
              key={pin.path}
              to={{ pathname: pathname, search: pinSearch(pin.path) }}
              aria-label={`Scope to ${displayNode(pin.path)}, ${pin.open} open items`}
              aria-current={filters.node === pin.path ? 'true' : undefined}
              className={cn(
                'flex items-center gap-2.5 rounded-md py-1.5 pr-2 pl-8 text-sm text-muted-foreground hover:bg-muted hover:text-foreground',
                filters.node === pin.path && 'bg-accent/10 font-medium text-foreground',
              )}
              title={`Scope every view to ${displayNode(pin.path)}`}
            >
              <Pin className="size-3.5 shrink-0" aria-hidden />
              <span className="truncate font-mono text-xs">{displayNode(pin.path)}</span>
              <Count value={pin.open} />
            </NavLink>
          ))}
        </Section>
        <Section id="tasks" label="Tasks" icon={ListTodo} collapsed={collapsed.has('tasks')} onToggle={() => toggle('tasks')}>
          <Entry to={routes.taskList} label="List" icon={List} count={counts.tasks} search={scope} nested />
          <Entry to={routes.kanban} label="Kanban" icon={Columns3} count={counts.open} search={scope} nested />
          <Entry to={routes.matrix} label="Size × impact" icon={Grid2x2} count={counts.toEstimate} search={scope} nested />
          <Entry to={routes.backlog} label="Backlog" icon={Inbox} count={counts.backlog} search={scope} nested />
        </Section>
        <Entry to={routes.stories} label="Stories" icon={BookOpen} count={counts.stories} search={scope} />
        <Section id="reflexions" label="Reflexions" icon={Lightbulb} collapsed={collapsed.has('reflexions')} onToggle={() => toggle('reflexions')}>
          <Entry to={routes.reflexions} end label="Board" icon={Columns3} count={counts.reflexions} search={scope} nested />
          <Entry to={routes.journal} label="Journal" icon={NotebookPen} count={counts.settled} search={scope} nested />
        </Section>
        <Entry to={routes.ressources} label="Ressources" icon={Library} count={counts.ressources} search={scope} />
        <div className="my-1 border-t border-border" />
        <Entry
          to={routes.health}
          label="Health"
          icon={HeartPulse}
          count={counts.errors}
          tone={counts.errors > 0 ? 'bg-red-500/15 text-red-700 dark:text-red-300' : undefined}
          search={scope}
        />
        <Entry
          to={routes.repo}
          label="Repository"
          icon={FolderGit2}
          count={repo?.uncommitted.length}
          tone={(repo?.uncommitted.length ?? 0) > 0 ? 'bg-amber-500/15 text-amber-700 dark:text-amber-300' : undefined}
          search={scope}
        />
        <Entry to={routes.people} label="People" icon={Users} search={scope} />
        <Entry to={routes.settings} label="Settings" icon={Settings} search={scope} />
      </nav>
    </aside>
  )
}
