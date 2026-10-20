import type { HTMLAttributes } from 'react'
import { cn } from '@/shared/lib'

export function Badge({ className, ...props }: HTMLAttributes<HTMLSpanElement>) {
  return (
    <span
      className={cn(
        'inline-flex items-center rounded-full border border-transparent px-2 py-0.5 text-xs font-medium whitespace-nowrap',
        className,
      )}
      {...props}
    />
  )
}
