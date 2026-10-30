import { ItemLink, useItems } from '@/entities/item'
import { Highlight } from '@/shared/ui'

// A `<node>#<id>` reference, linked to its item when the tree knows it.
export function ItemReference({ reference }: { reference: string }) {
  const { data: items = [] } = useItems()
  const item = items.find((entry) => entry.reference === reference)
  const label = (
    <span className="font-mono text-[11px]" title={item?.title ?? undefined}>
      <Highlight text={reference} />
    </span>
  )
  return item ? (
    <ItemLink item={item} className="rounded bg-violet-500/15 px-1.5 py-0.5 text-violet-800 dark:text-violet-200">
      {label}
    </ItemLink>
  ) : (
    label
  )
}

export function References({ references }: { references: string[] }) {
  if (references.length === 0) {
    return <span className="text-xs text-muted-foreground">—</span>
  }
  return (
    <ul className="flex flex-wrap gap-1">
      {references.map((reference) => (
        <li key={reference}>
          <ItemReference reference={reference} />
        </li>
      ))}
    </ul>
  )
}
