import { useState } from 'react'
import { isColor } from '@/entities/theme'
import { Input } from '@/shared/ui'
import './settings.css'

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
    <div className="color-field">
      <label htmlFor={`${id}-hex`} className="color-field__label">
        {label}
      </label>
      <p className="color-field__hint">{hint}</p>
      <div className="color-field__controls">
        <input
          type="color"
          aria-label={`${label}, color picker`}
          value={value}
          onChange={(event) => onChange(event.target.value)}
          className="color-field__picker"
        />
        <Input
          id={`${id}-hex`}
          value={text}
          spellCheck={false}
          aria-invalid={!isColor(text)}
          className="color-field__hex"
          onChange={(event) => {
            setText(event.target.value)
            if (isColor(event.target.value)) {
              onChange(event.target.value.toLowerCase())
            }
          }}
        />
        {!isColor(text) && <span className="color-field__error">#rrggbb</span>}
      </div>
    </div>
  )
}
