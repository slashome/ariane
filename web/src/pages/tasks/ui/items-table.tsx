import { useMemo, useState } from 'react'
import { idColumn, statusColumn, titleColumn, useMoveItem } from '@/entities/item'
import { statusLabel, useNodes } from '@/entities/node'
import { enumValues, kindLabel, useSchema } from '@/entities/schema'
import type { Item, NodeSummary, Schema } from '@/shared/api'
import { displayNode } from '@/shared/lib'
import { DataTable, type Column, type Reorder } from '@/shared/ui'
import { ItemDetails } from '@/widgets/item-details'

type RankedKey = 'priority' | 'size' | 'impact'

function rank(schema: Schema | undefined, key: RankedKey, value: string | null): number | null {
  const index = value === null ? -1 : enumValues(schema, key).indexOf(value)
  return index < 0 ? null : index
}

function positions(items: Item[]): Map<string, number> {
  const counts = new Map<string, number>()
  return new Map(
    items.map((item) => {
      const group = `${item.node}#${item.kind}`
      const position = (counts.get(group) ?? 0) + 1
      counts.set(group, position)
      return [item.path, position]
    }),
  )
}

function buildColumns(schema: Schema | undefined, nodes: NodeSummary[], order: Map<string, number>, index: Map<string, number>): Column<Item>[] {
  const itself = (item: Item) => item
  const ranked = (key: RankedKey, header: string): Column<Item> => ({
    id: key,
    header,
    sortValue: (item) => rank(schema, key, item[key]),
    cell: (item) => item[key] ?? <span className="text-muted-foreground">—</span>,
  })
  return [
    {
      id: 'order',
      header: 'Order',
      sortValue: (item) => index.get(item.path),
      className: 'font-mono text-xs text-muted-foreground',
      cell: (item) => order.get(item.path),
    },
    {
      id: 'node',
      header: 'Node',
      sortValue: (item) => item.node,
      className: 'font-mono text-xs',
      cell: (item) => displayNode(item.node),
    },
    idColumn(itself),
    titleColumn(itself, { link: false }),
    { id: 'kind', header: 'Kind', sortValue: (item) => kindLabel(item.kind), cell: (item) => kindLabel(item.kind) },
    statusColumn(itself, (item) => statusLabel(nodes, item.node, item.status)),
    ranked('priority', 'Priority'),
    ranked('size', 'Size'),
    ranked('impact', 'Impact'),
    {
      id: 'created',
      header: 'Created',
      sortValue: (item) => item.created,
      className: 'font-mono text-xs whitespace-nowrap',
      cell: (item) => item.created ?? '—',
    },
  ]
}

export function ItemsTable({ items }: { items: Item[] }) {
  const { data: schema } = useSchema()
  const { data: nodes = [] } = useNodes()
  const order = useMemo(() => positions(items), [items])
  const index = useMemo(() => new Map(items.map((item, position) => [item.path, position])), [items])
  const columns = useMemo(() => buildColumns(schema, nodes, order, index), [schema, nodes, order, index])
  const [selected, setSelected] = useState<Item | null>(null)
  const reorder: Reorder<Item> = { column: 'order', onMove: useMoveItem() }

  return (
    <>
      <DataTable
        rows={items}
        columns={columns}
        rowKey={(item) => item.path}
        storageKey="tasks"
        reorder={reorder}
        label="Tasks"
        empty="No item matches."
        rowProps={(item) => ({
          tabIndex: 0,
          onClick: () => setSelected(item),
          onKeyDown: (event) => {
            if (event.key === 'Enter') {
              setSelected(item)
            }
          },
          className: 'cursor-pointer hover:bg-muted/50 focus-visible:bg-muted focus-visible:outline-none',
        })}
      />
      <ItemDetails item={selected} onClose={() => setSelected(null)} />
    </>
  )
}
