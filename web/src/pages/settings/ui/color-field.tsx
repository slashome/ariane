import { useState } from 'react'
import { isColor } from '@/entities/theme'
import { Input } from '@/shared/ui'

export function ColorField({
  id,
  label,
  hint,
  value,
  onChange,
}: {
  id: string
  label: string
  hint: string
  value: string
  onChange: (color: string) => void
}) {
  // The text field may hold an unfinished color; the theme only takes complete ones.
  const [text, setText] = useState(value)
  const [last, setLast] = useState(value)
  if (value !== last) {
    setLast(value)
    setText(value)
  }
  return (
    <div className="space-y-1.5">
      <label htmlFor={`${id}-hex`} className="text-sm font-medium">
        {label}
      </label>
      <p className="text-xs text-muted-foreground">{hint}</p>
      <div className="flex items-center gap-2">
        <input
          type="color"
          aria-label={`${label}, color picker`}
          value={value}
          onChange={(event) => onChange(event.target.value)}
          className="size-9 cursor-pointer rounded-md border border-border bg-background p-0.5"
        />
        <Input
          id={`${id}-hex`}
          value={text}
          spellCheck={false}
          aria-invalid={!isColor(text)}
          className="w-28 font-mono"
          onChange={(event) => {
            setText(event.target.value)
            if (isColor(event.target.value)) {
              onChange(event.target.value.toLowerCase())
            }
          }}
        />
        {!isColor(text) && <span className="text-xs text-red-700 dark:text-red-300">#rrggbb</span>}
      </div>
    </div>
  )
}
