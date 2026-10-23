import { CornerDownLeft } from 'lucide-react'
import { useEffect, useRef } from 'react'
import { cn } from '@/shared/lib'
import { Highlight } from '@/shared/ui'
import type { Entry } from '../model/rank'

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
  const Icon = command.icon
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
      className={cn(
        'flex cursor-pointer items-start gap-3 rounded-md px-3 py-2 text-sm',
        active ? 'bg-accent/15 text-foreground' : 'text-foreground/90',
      )}
    >
      {Icon && <Icon className="mt-0.5 size-4 shrink-0 text-muted-foreground" aria-hidden />}
      <span className="min-w-0 flex-1">
        <span className="block truncate">
          <Highlight text={command.title} ranges={entry.ranges} />
          {command.children && <span className="text-muted-foreground"> ›</span>}
        </span>
        {command.detail && (
          <span className="mt-0.5 line-clamp-2 block text-xs leading-relaxed text-muted-foreground">
            <Highlight text={command.detail.text} ranges={command.detail.ranges} />
          </span>
        )}
      </span>
      {command.hint && <span className="shrink-0 truncate font-mono text-[11px] text-muted-foreground">{command.hint}</span>}
      {active && <CornerDownLeft className="mt-0.5 size-3.5 shrink-0 text-muted-foreground" aria-hidden />}
    </li>
  )
}
