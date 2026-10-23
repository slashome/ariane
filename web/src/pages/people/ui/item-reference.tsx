import { ItemLink, useItems } from '@/entities/item'

// A `<node>#<id>` reference, linked to its item when the tree knows it.
export function ItemReference({ reference }: { reference: string }) {
  const { data: items = [] } = useItems()
  const item = items.find((entry) => entry.reference === reference)
  const label = <span className="font-mono text-[11px]">{reference}</span>
  return item ? (
    <ItemLink item={item} className="rounded bg-violet-500/15 px-1.5 py-0.5 text-violet-800 dark:text-violet-200">
      {label}
    </ItemLink>
  ) : (
    label
  )
}
