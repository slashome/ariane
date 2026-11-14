import { ItemCard } from '@/entities/item'
import type { Item, Kind } from '@/shared/api'
import { cn } from '@/shared/lib'
import { Draggable } from '@/shared/ui'
import type { OutlineEntry } from '../model/release-board'

const kindMarks: Partial<Record<Kind, { letter: string; label: string; tone: string }>> = {
  epic: { letter: 'E', label: 'Epic', tone: 'bg-violet-500/15 text-violet-700 dark:text-violet-300' },
  story: { letter: 'S', label: 'Story', tone: 'bg-sky-500/15 text-sky-700 dark:text-sky-300' },
  task: { letter: 'T', label: 'Task', tone: 'bg-muted text-muted-foreground' },
}

export function KindMark({ kind }: { kind: Kind }) {
  const mark = kindMarks[kind]
  if (!mark) {
    return null
  }
  return (
    <span title={mark.label} aria-label={mark.label} className={cn('shrink-0 rounded px-1 font-mono text-[10px] font-semibold', mark.tone)}>
      {mark.letter}
    </span>
  )
}

export function ReleaseCard({ item, showNode, className }: { item: Item; showNode: boolean; className?: string }) {
  return <ItemCard item={item} showNode={showNode} aside={<KindMark kind={item.kind} />} className={cn('p-2', className)} />
}

const indents = ['', 'ml-3', 'ml-6']

export function ReleaseOutline({
  entries,
  scope,
  onOpen,
  empty,
}: {
  entries: OutlineEntry[]
  scope: string
  onOpen: (item: Item) => void
  empty: string
}) {
  if (entries.length === 0) {
    return <p className="px-2 py-6 text-center text-xs text-muted-foreground">{empty}</p>
  }
  return (
    <ul className="flex flex-col gap-1.5">
      {entries.map((entry) =>
        entry.type === 'heading' ? (
          <li key={`heading:${entry.reference}`} className={cn('truncate pt-1 text-[11px] text-muted-foreground', indents[entry.depth])}>
            {entry.label}
          </li>
        ) : (
          <li key={entry.item.path} className={indents[entry.depth]}>
            <Draggable id={entry.item.path} label={`${entry.item.id ?? ''} ${entry.item.title ?? entry.item.path}`} onOpen={() => onOpen(entry.item)}>
              <ReleaseCard item={entry.item} showNode={entry.item.node !== scope} />
            </Draggable>
          </li>
        ),
      )}
    </ul>
  )
}
