import { AlertTriangle, Search } from 'lucide-react'
import { useId, useRef } from 'react'
import { cn } from '@/shared/lib'
import { Input, Tooltip } from '@/shared/ui'
import { useQueryField } from '../model/use-query-field'

// The search field of every item view, bound to `?q=`: typed filters, free words, suggestions for the
// token under the cursor, and what it does not understand reported beside it. With `count`, it tells
// how many items the view shows.
export function QueryField({ count, total, className }: { count?: number; total?: number; className?: string }) {
  const input = useRef<HTMLInputElement>(null)
  const field = useQueryField(input)
  const id = useId()
  const listbox = `${id}-suggestions`
  const described = `${id}-issues`
  const errors = field.issues.some((issue) => issue.severity === 'error')
  const expanded = field.options.length > 0

  return (
    <div className={cn('relative w-80', className)}>
      <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" aria-hidden />
      <Input
        ref={input}
        type="search"
        role="combobox"
        aria-label="Search items"
        aria-autocomplete="list"
        aria-expanded={expanded}
        aria-controls={expanded ? listbox : undefined}
        aria-activedescendant={expanded ? `${listbox}-${field.active}` : undefined}
        aria-invalid={errors || undefined}
        aria-describedby={field.issues.length > 0 ? described : undefined}
        autoComplete="off"
        spellCheck={false}
        placeholder="Search, or status: size:<=M @person…"
        className={cn('pl-8', count !== undefined && 'pr-24', errors && 'border-red-500/60 focus-visible:outline-red-500')}
        value={field.text}
        onChange={(event) => field.onChange(event.target.value, event.target.selectionStart)}
        onSelect={field.onSelect}
        onKeyDown={field.onKeyDown}
        onFocus={field.onFocus}
        onBlur={field.onBlur}
      />
      <div className="pointer-events-none absolute top-1/2 right-2.5 flex -translate-y-1/2 items-center gap-1.5 text-xs text-muted-foreground">
        {count !== undefined && (
          <span aria-live="polite" className="tabular-nums">
            {total === undefined || field.text.trim() === '' ? `${count} items` : `${count} of ${total}`}
          </span>
        )}
        {field.issues.length > 0 && (
          <Tooltip content={field.issues.map((issue) => issue.message).join('\n')}>
            <span className="pointer-events-auto" tabIndex={0} aria-label={`${field.issues.length} problems in the search`}>
              <AlertTriangle className={cn('size-3.5', errors ? 'text-red-500' : 'text-amber-500')} aria-hidden />
            </span>
          </Tooltip>
        )}
      </div>
      <p id={described} className="sr-only">
        {field.issues.map((issue) => issue.message).join('. ')}
      </p>
      {field.open && (expanded || field.issues.length > 0) && (
        <div className="absolute top-full right-0 left-0 z-40 mt-1 rounded-md border border-border bg-popover p-1 text-sm shadow-lg">
          {field.issues.map((issue) => (
            <p
              key={`${issue.start}-${issue.message}`}
              className={cn('px-2 py-1 text-xs', issue.severity === 'error' ? 'text-red-600 dark:text-red-400' : 'text-amber-700 dark:text-amber-300')}
            >
              {issue.message}
            </p>
          ))}
          <ul id={listbox} role="listbox" aria-label="Suggestions" className="max-h-72 overflow-y-auto">
            {field.options.map((option, index) => (
              <li
                key={option.value}
                id={`${listbox}-${index}`}
                role="option"
                aria-selected={index === field.active}
                onMouseDown={(event) => {
                  event.preventDefault()
                  field.choose(option)
                }}
                className={cn(
                  'flex cursor-pointer items-baseline justify-between gap-3 rounded px-2 py-1.5',
                  index === field.active ? 'bg-accent/15 text-foreground' : 'text-muted-foreground hover:bg-muted',
                )}
              >
                <span className="font-mono text-xs text-foreground">{option.label}</span>
                {option.description && <span className="truncate text-xs">{option.description}</span>}
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  )
}
