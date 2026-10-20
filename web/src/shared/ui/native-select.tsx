import type { SelectHTMLAttributes } from 'react'
import { cn } from '@/shared/lib'

export function NativeSelect({ className, ...props }: SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <select
      className={cn(
        'h-9 rounded-md border border-border bg-background px-2 text-sm focus-visible:outline-2 focus-visible:outline-ring',
        className,
      )}
      {...props}
    />
  )
}
