import { ExternalLink, Maximize2 } from 'lucide-react'
import type { ReactNode } from 'react'
import { ItemLink, StatusBadge } from '@/entities/item'
import { externalUrl, statusLabel, trackerOf, useNodes } from '@/entities/node'
import { kindLabel } from '@/entities/schema'
import type { Item, Tracker } from '@/shared/api'
import { displayNode } from '@/shared/lib'
import { Sheet } from '@/shared/ui'

function Field({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div className="grid grid-cols-[7rem_1fr] gap-2 text-sm">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="min-w-0 break-words">{value ?? <span className="text-muted-foreground">—</span>}</dd>
    </div>
  )
}

function ExternalLinks({ entries, tracker }: { entries: string[]; tracker: Tracker | null }) {
  return (
    <ul className="space-y-0.5">
      {entries.map((entry) => {
        const url = externalUrl(tracker, entry)
        return (
          <li key={entry} className="font-mono text-xs">
            {url ? (
              <a href={url} target="_blank" rel="noreferrer" className="inline-flex items-center gap-1 text-primary underline-offset-2 hover:underline">
                {entry}
                <ExternalLink className="size-3" aria-hidden />
              </a>
            ) : (
              entry
            )}
          </li>
        )
      })}
    </ul>
  )
}

export function ItemDetails({ item, onClose }: { item: Item | null; onClose: () => void }) {
  const { data: nodes = [] } = useNodes()
  return (
    <Sheet
      open={item !== null}
      onOpenChange={(open) => !open && onClose()}
      title={item?.title ?? item?.path ?? ''}
      description={item ? `${displayNode(item.node)} · ${item.id ?? 'no id'}` : undefined}
    >
      {item && item.id !== null && (
        <ItemLink item={item} className="inline-flex w-fit items-center gap-1.5 text-sm text-primary">
          <Maximize2 className="size-3.5" aria-hidden />
          Open the page: body, parent, children, dependencies
        </ItemLink>
      )}
      {item && (
        <dl className="space-y-2">
          <Field label="Status" value={<StatusBadge status={item.status} category={item.category} label={statusLabel(nodes, item.node, item.status)} />} />
          <Field label="Kind" value={kindLabel(item.kind)} />
          <Field label="Archived" value={item.archived ? 'yes' : 'no'} />
          <Field label="Priority" value={item.priority} />
          <Field label="Size" value={item.size} />
          <Field label="Impact" value={item.impact} />
          <Field label="Created" value={item.created} />
          <Field label="External" value={
              item.external.length > 0 ? (
                <ExternalLinks entries={item.external} tracker={trackerOf(nodes, item.node)} />
              ) : null
            }
          />
          <Field label="Schema" value={item.version === null ? 'not versioned' : `version ${item.version}`} />
          <Field label="Path" value={<code className="font-mono text-xs">{item.path}</code>} />
        </dl>
      )}
    </Sheet>
  )
}
