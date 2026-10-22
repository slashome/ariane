import type { LucideIcon } from 'lucide-react'
import type { ReactNode } from 'react'
import { cn } from '@/shared/lib'

export function EmptyState({
  icon: Icon,
  title,
  children,
  className,
}: {
  icon?: LucideIcon
  title: string
  children?: ReactNode
  className?: string
}) {
  return (
    <div className={cn('flex flex-col items-center gap-1 rounded-lg border border-dashed border-border p-10 text-center text-sm text-muted-foreground', className)}>
      {Icon && <Icon className="mb-1 size-6" aria-hidden />}
      <p className="font-medium text-foreground">{title}</p>
      {children}
    </div>
  )
}

export function LoadingState({ rows = 4, label = 'Loading…' }: { rows?: number; label?: string }) {
  return (
    <div role="status" aria-label={label} className="space-y-2">
      {Array.from({ length: rows }, (_, index) => (
        <div key={index} className="h-10 animate-pulse rounded-md bg-muted" style={{ opacity: 1 - index * (0.6 / rows) }} />
      ))}
    </div>
  )
}

export function ErrorState({ message }: { message: string }) {
  return (
    <p role="alert" className="rounded-md border border-red-500/40 bg-red-500/10 p-3 text-sm">
      {message}
    </p>
  )
}

export function PageHeader({
  title,
  description,
  actions,
  children,
}: {
  title: ReactNode
  description?: ReactNode
  actions?: ReactNode
  children?: ReactNode
}) {
  return (
    <div className="flex flex-wrap items-end justify-between gap-3">
      <div className="min-w-0">
        <h1 className="text-xl font-semibold">{title}</h1>
        {description && <p className="text-sm text-muted-foreground">{description}</p>}
        {children}
      </div>
      {actions && <div className="flex flex-wrap items-center gap-2">{actions}</div>}
    </div>
  )
}

export function Kbd({ children }: { children: ReactNode }) {
  return (
    <kbd className="rounded border border-border bg-muted px-1 py-px font-mono text-[10px] text-muted-foreground">{children}</kbd>
  )
}
