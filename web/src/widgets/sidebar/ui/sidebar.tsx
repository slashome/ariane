import { Columns3, Grid2x2, LayoutDashboard, ListTodo, type LucideIcon } from 'lucide-react'
import { NavLink, useLocation } from 'react-router'
import { cn } from '@/shared/lib'

interface Entry {
  to: string
  label: string
  icon: LucideIcon
}

const entries: Entry[] = [
  { to: '/dashboard', label: 'Dashboard', icon: LayoutDashboard },
  { to: '/tasks', label: 'Tasks', icon: ListTodo },
  { to: '/kanban', label: 'Kanban', icon: Columns3 },
  { to: '/matrix', label: 'Size × impact', icon: Grid2x2 },
]

export function Sidebar() {
  const { search } = useLocation()
  return (
    <aside className="flex w-56 shrink-0 flex-col border-r border-border bg-sidebar">
      <div className="flex h-14 items-center gap-2 border-b border-border px-4 font-semibold">
        <span className="size-2.5 rounded-full bg-primary" aria-hidden />
        Ariane
      </div>
      <nav aria-label="Main" className="flex flex-col gap-0.5 p-2">
        {entries.map(({ to, label, icon: Icon }) => (
          <NavLink
            key={to}
            to={{ pathname: to, search }}
            className={({ isActive }) =>
              cn(
                'flex items-center gap-2.5 rounded-md px-3 py-2 text-sm text-muted-foreground hover:bg-muted hover:text-foreground',
                isActive && 'bg-muted font-medium text-foreground',
              )
            }
          >
            <Icon className="size-4" aria-hidden />
            {label}
          </NavLink>
        ))}
      </nav>
    </aside>
  )
}
