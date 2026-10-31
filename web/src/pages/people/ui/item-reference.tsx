import { ItemLink, useItems } from '@/entities/item'
import { Highlight } from '@/shared/ui'
import './people.css'

// A `<node>#<id>` reference, linked to its item when the tree knows it.
export function ItemReference({ reference }: { reference: string }) {
  const { data: items = [] } = useItems()
  const item = items.find((entry) => entry.reference === reference)
  const label = (
    <span className="item-reference" title={item?.title ?? undefined}>
      <Highlight text={reference} />
    </span>
  )
  return item ? (
    <ItemLink item={item} className="item-reference__link">
      {label}
    </ItemLink>
  ) : (
    label
  )
}

export function References({ references }: { references: string[] }) {
  if (references.length === 0) {
    return <span className="item-references__none">—</span>
  }
  return (
    <ul className="item-references">
      {references.map((reference) => (
        <li key={reference}>
          <ItemReference reference={reference} />
        </li>
      ))}
    </ul>
  )
}
