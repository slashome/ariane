import { useMemo, useState } from 'react'
import { idColumn, statusColumn, titleColumn } from '@/entities/item'
import { statusLabel, useNodes } from '@/entities/node'
import { enumValues, kindLabel, useSchema } from '@/entities/schema'
import type { Item, NodeSummary, Schema } from '@/shared/api'
import { displayNode } from '@/shared/lib'
import { DataTable, type Column } from '@/shared/ui'
import { ItemDetails } from '@/widgets/item-details'
import './tasks.css'

type RankedKey = 'priority' | 'size' | 'impact'

function rank(schema: Schema | undefined, key: RankedKey, value: string | null): number | null {
  const index = value === null ? -1 : enumValues(schema, key).indexOf(value)
  return index < 0 ? null : index
}

function buildColumns(schema: Schema | undefined, nodes: NodeSummary[]): Column<Item>[] {
  const itself = (item: Item) => item
  const ranked = (key: RankedKey, header: string): Column<Item> => ({
    id: key,
    header,
    sortValue: (item) => rank(schema, key, item[key]),
    cell: (item) => item[key] ?? <span className="items-table__none">—</span>,
  })
  return [
    {
      id: 'node',
      header: 'Node',
      sortValue: (item) => item.node,
      className: 'items-table__node',
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
      className: 'items-table__created',
      cell: (item) => item.created ?? '—',
    },
  ]
}

export function ItemsTable({ items }: { items: Item[] }) {
  const { data: schema } = useSchema()
  const { data: nodes = [] } = useNodes()
  const columns = useMemo(() => buildColumns(schema, nodes), [schema, nodes])
  const [selected, setSelected] = useState<Item | null>(null)

  return (
    <>
      <DataTable
        rows={items}
        columns={columns}
        rowKey={(item) => item.path}
        storageKey="tasks"
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
          className: 'items-table__row',
        })}
      />
      <ItemDetails item={selected} onClose={() => setSelected(null)} />
    </>
  )
}
