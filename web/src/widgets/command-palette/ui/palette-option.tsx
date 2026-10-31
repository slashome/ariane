import { useEffect, useRef } from 'react'
import { cn } from '@/shared/lib'
import { Highlight, Icon } from '@/shared/ui'
import type { Entry } from '../model/rank'
import './command-palette.css'

// One command of the list: its icon, its title with the words found, an excerpt under it, a hint at the right.
export function PaletteOption({
  id,
  entry,
  active,
  onHover,
  onChoose,
}: {
  id: string
  entry: Entry
  active: boolean
  onHover: () => void
  onChoose: (shift: boolean) => void
}) {
  const { command } = entry
  const element = useRef<HTMLLIElement>(null)
  useEffect(() => {
    if (active) {
      element.current?.scrollIntoView({ block: 'nearest' })
    }
  }, [active])
  return (
    <li
      ref={element}
      id={id}
      role="option"
      aria-selected={active}
      onMouseMove={onHover}
      onMouseDown={(event) => event.preventDefault()}
      onClick={(event) => onChoose(event.shiftKey)}
      className={cn('palette-option', active && 'palette-option--active')}
    >
      {command.icon && <Icon name={command.icon} className="palette-option__icon" />}
      <span className="palette-option__body">
        <span className="palette-option__title lu-truncate">
          <Highlight text={command.title} ranges={entry.ranges} />
          {command.children && <span className="palette-option__more"> ›</span>}
        </span>
        {command.detail && (
          <span className="palette-option__detail lu-line-clamp">
            <Highlight text={command.detail.text} ranges={command.detail.ranges} />
          </span>
        )}
      </span>
      {command.hint && <span className="palette-option__hint lu-truncate">{command.hint}</span>}
      {active && <Icon name="enter" className="palette-option__enter" />}
    </li>
  )
}
