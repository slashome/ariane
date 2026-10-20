import type { ReactNode } from 'react'
import { kindLabel } from '@/entities/schema'
import type { Item } from '@/shared/api'
import { displayNode } from '@/shared/lib'
import { Sheet } from '@/shared/ui'
import { StatusBadge } from './status-badge'

function Field({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div className="grid grid-cols-[7rem_1fr] gap-2 text-sm">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="min-w-0 break-words">{value ?? <span className="text-muted-foreground">—</span>}</dd>
    </div>
  )
}

export function ItemDetails({ item, onClose }: { item: Item | null; onClose: () => void }) {
  return (
    <Sheet
      open={item !== null}
      onOpenChange={(open) => !open && onClose()}
      title={item?.title ?? item?.path ?? ''}
      description={item ? `${displayNode(item.node)} · ${item.id ?? 'no id'}` : undefined}
    >
      {item && (
        <dl className="space-y-2">
          <Field label="Status" value={<StatusBadge status={item.status} category={item.category} />} />
          <Field label="Kind" value={kindLabel(item.kind)} />
          <Field label="Archived" value={item.archived ? 'yes' : 'no'} />
          <Field label="Priority" value={item.priority} />
          <Field label="Size" value={item.size} />
          <Field label="Impact" value={item.impact} />
          <Field label="Created" value={item.created} />
          <Field label="Schema" value={item.version === null ? 'not versioned' : `version ${item.version}`} />
          <Field label="Path" value={<code className="font-mono text-xs">{item.path}</code>} />
        </dl>
      )}
    </Sheet>
  )
}
