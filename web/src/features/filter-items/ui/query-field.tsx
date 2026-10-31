import { useId, useRef } from 'react'
import { cn } from '@/shared/lib'
import { Icon, Input, Tooltip } from '@/shared/ui'
import { useQueryField } from '../model/use-query-field'
import './filter-items.css'

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
    <div className={cn('query-field', className)}>
      <Icon name="search" className="query-field__icon" />
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
        className={cn('query-field__input', count !== undefined && 'query-field__input--counted', errors && 'query-field__input--invalid')}
        value={field.text}
        onChange={(event) => field.onChange(event.target.value, event.target.selectionStart)}
        onSelect={field.onSelect}
        onKeyDown={field.onKeyDown}
        onFocus={field.onFocus}
        onBlur={field.onBlur}
      />
      <div className="query-field__status">
        {count !== undefined && (
          <span aria-live="polite" className="query-field__count">
            {total === undefined || field.text.trim() === '' ? `${count} items` : `${count} of ${total}`}
          </span>
        )}
        {field.issues.length > 0 && (
          <Tooltip content={field.issues.map((issue) => issue.message).join('\n')}>
            <span className="query-field__issues" tabIndex={0} aria-label={`${field.issues.length} problems in the search`}>
              <Icon name="warning" className={cn('query-field__warning', errors && 'query-field__warning--error')} />
            </span>
          </Tooltip>
        )}
      </div>
      <p id={described} className="lu-visually-hidden">
        {field.issues.map((issue) => issue.message).join('. ')}
      </p>
      {field.open && (expanded || field.issues.length > 0) && (
        <div className="query-field__popup">
          {field.issues.map((issue) => (
            <p
              key={`${issue.start}-${issue.message}`}
              className={cn('query-field__issue', issue.severity === 'error' && 'query-field__issue--error')}
            >
              {issue.message}
            </p>
          ))}
          <ul id={listbox} role="listbox" aria-label="Suggestions" className="query-field__options">
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
                className={cn('query-field__option', index === field.active && 'query-field__option--active')}
              >
                <span className="query-field__option-label">{option.label}</span>
                {option.description && <span className="lu-truncate query-field__option-description">{option.description}</span>}
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  )
}
