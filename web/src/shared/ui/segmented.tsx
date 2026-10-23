import { cn } from '@/shared/lib'

// A row of toggle buttons for one header field; pressing the current value again unsets it.
export function Segmented({
  label,
  values,
  value,
  onChange,
  focusable = true,
}: {
  label: string
  values: string[]
  value: string | null
  onChange: (value: string | null) => void
  // False where the keyboard is handled around it, as in the backlog's rows.
  focusable?: boolean
}) {
  return (
    <div role="group" aria-label={label} className="inline-flex overflow-hidden rounded-md border border-border">
      {values.map((entry) => (
        <button
          key={entry}
          type="button"
          tabIndex={focusable ? undefined : -1}
          aria-pressed={value === entry}
          onClick={(event) => {
            event.stopPropagation()
            onChange(value === entry ? null : entry)
          }}
          className={cn(
            'px-1.5 py-0.5 text-[11px] text-muted-foreground hover:bg-muted',
            value === entry && 'bg-primary text-primary-foreground hover:bg-primary',
          )}
        >
          {entry}
        </button>
      ))}
    </div>
  )
}
