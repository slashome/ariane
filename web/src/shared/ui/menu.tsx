import { DropdownMenu } from 'radix-ui'
import type { ReactNode } from 'react'
import { cn } from '@/shared/lib'

export interface MenuEntry {
  key: string
  label: string
  disabled?: boolean
  hint?: string | null
  destructive?: boolean
  onSelect: () => void
}

export function Menu({ label, trigger, entries }: { label: string; trigger: ReactNode; entries: MenuEntry[] }) {
  return (
    <DropdownMenu.Root modal={false}>
      <DropdownMenu.Trigger asChild aria-label={label}>
        {trigger}
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="end"
          sideOffset={4}
          aria-label={label}
          className="z-50 min-w-48 max-w-72 rounded-md border border-border bg-background p-1 text-sm shadow-lg"
        >
          {entries.map((entry) => (
            <DropdownMenu.Item
              key={entry.key}
              disabled={entry.disabled}
              onSelect={entry.onSelect}
              className={cn(
                'flex cursor-default flex-col rounded px-2 py-1.5 outline-none select-none data-[highlighted]:bg-muted data-[disabled]:opacity-60',
                entry.destructive && !entry.disabled && 'text-red-600 dark:text-red-400',
              )}
            >
              <span>{entry.label}</span>
              {entry.hint && <span className="text-[11px] text-muted-foreground">{entry.hint}</span>}
            </DropdownMenu.Item>
          ))}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  )
}
