import { Check, ChevronDown } from 'lucide-react'
import { Select as SelectPrimitive } from 'radix-ui'
import type { ReactNode } from 'react'
import { cn } from '@/shared/lib'

// A select whose list the page draws, unlike NativeSelect, whose list the operating system draws in
// its own font: use it when the options need their own layout, such as a tree. Keyboard, typeahead
// and screen-reader roles are Radix's. A value may not be the empty string.
export function Select({
  label,
  value,
  onValueChange,
  display,
  children,
  className,
}: {
  label: string
  value: string
  onValueChange: (value: string) => void
  // What the closed select shows; the selected option's own text otherwise.
  display?: ReactNode
  children: ReactNode
  className?: string
}) {
  return (
    <SelectPrimitive.Root value={value} onValueChange={onValueChange}>
      <SelectPrimitive.Trigger
        aria-label={label}
        className={cn(
          'inline-flex h-9 items-center justify-between gap-2 rounded-md border border-border bg-background pr-2.5 pl-2.5 text-left text-sm focus-visible:outline-2 focus-visible:outline-ring',
          className,
        )}
      >
        <span className="min-w-0 truncate">
          <SelectPrimitive.Value>{display}</SelectPrimitive.Value>
        </span>
        <SelectPrimitive.Icon asChild>
          <ChevronDown className="size-4 shrink-0 text-muted-foreground" aria-hidden />
        </SelectPrimitive.Icon>
      </SelectPrimitive.Trigger>
      <SelectPrimitive.Portal>
        <SelectPrimitive.Content
          position="popper"
          sideOffset={4}
          className="z-50 max-h-[min(32rem,var(--radix-select-content-available-height))] min-w-(--radix-select-trigger-width) overflow-hidden rounded-md border border-border bg-popover text-sm shadow-lg"
        >
          <SelectPrimitive.Viewport className="p-1">{children}</SelectPrimitive.Viewport>
        </SelectPrimitive.Content>
      </SelectPrimitive.Portal>
    </SelectPrimitive.Root>
  )
}

export function SelectItem({
  value,
  textValue,
  children,
  className,
}: {
  value: string
  // The text typeahead matches and the closed select shows, when `children` is more than text.
  textValue?: string
  children: ReactNode
  className?: string
}) {
  return (
    <SelectPrimitive.Item
      value={value}
      textValue={textValue}
      className={cn(
        'relative flex cursor-pointer items-center rounded py-1 pr-2 pl-7 outline-none select-none data-[highlighted]:bg-muted data-[state=checked]:font-medium',
        className,
      )}
    >
      <SelectPrimitive.ItemIndicator className="absolute left-2 inline-flex">
        <Check className="size-3.5" aria-hidden />
      </SelectPrimitive.ItemIndicator>
      <SelectPrimitive.ItemText>{children}</SelectPrimitive.ItemText>
    </SelectPrimitive.Item>
  )
}
