import { ChevronDown } from 'lucide-react'
import type { SelectHTMLAttributes } from 'react'
import { cn } from '@/shared/lib'

// A native select drawn without the browser's arrow, so the chevron keeps the same padding as the text
// on every platform; the keyboard and the option list stay the browser's.
export function NativeSelect({ className, ...props }: SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <span className="relative inline-flex">
      <select
        className={cn(
          'h-9 w-full appearance-none rounded-md border border-border bg-background pr-8 pl-2.5 text-sm focus-visible:outline-2 focus-visible:outline-ring',
          className,
        )}
        {...props}
      />
      <ChevronDown className="pointer-events-none absolute top-1/2 right-2.5 size-4 -translate-y-1/2 text-muted-foreground" aria-hidden />
    </span>
  )
}
