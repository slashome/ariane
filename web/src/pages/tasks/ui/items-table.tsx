import {
  createColumnHelper,
  createSortedRowModel,
  rowSortingFeature,
  sortFn_alphanumeric,
  sortFn_basic,
  tableFeatures,
  useTable,
} from '@tanstack/react-table'
import { ArrowDown, ArrowUp, ArrowUpDown } from 'lucide-react'
import { useMemo, useState } from 'react'
import { StatusBadge } from '@/entities/item'
import { statusLabel, useNodes } from '@/entities/node'
import { enumValues, kindLabel, useSchema } from '@/entities/schema'
import type { Item, NodeSummary, Schema } from '@/shared/api'
import { cn, displayNode } from '@/shared/lib'
import { Highlight } from '@/shared/ui'
import { ItemDetails } from '@/widgets/item-details'

const features = tableFeatures({
  rowSortingFeature,
  sortedRowModel: createSortedRowModel(),
  sortFns: { alphanumeric: sortFn_alphanumeric, basic: sortFn_basic },
})

const helper = createColumnHelper<typeof features, Item>()

function rank(schema: Schema | undefined, key: string, value: string | null): number | undefined {
  const index = value === null ? -1 : enumValues(schema, key).indexOf(value)
  return index < 0 ? undefined : index
}

function buildColumns(schema: Schema | undefined, nodes: NodeSummary[]) {
  const text = (value: string | null) => value ?? undefined
  const ranked = (key: 'priority' | 'size' | 'impact', header: string) =>
    helper.accessor((item) => rank(schema, key, item[key]), {
      id: key,
      header,
      sortFn: 'basic',
      sortDescFirst: false,
      sortUndefined: 'last',
      cell: ({ row }) => row.original[key] ?? <span className="text-muted-foreground">—</span>,
    })
  return helper.columns([
    helper.accessor((item) => item.node, {
      id: 'node',
      header: 'Node',
      sortFn: 'alphanumeric',
      cell: ({ row }) => <span className="font-mono text-xs">{displayNode(row.original.node)}</span>,
    }),
    helper.accessor((item) => text(item.id), {
      id: 'id',
      header: 'Id',
      sortFn: 'alphanumeric',
      sortUndefined: 'last',
      cell: ({ row }) => (
        <span className="font-mono text-xs">
          <Highlight text={row.original.id ?? '—'} />
        </span>
      ),
    }),
    helper.accessor((item) => text(item.title), {
      id: 'title',
      header: 'Title',
      sortFn: 'alphanumeric',
      sortUndefined: 'last',
      cell: ({ row }) => (
        <span className={cn('line-clamp-2', row.original.archived && 'text-muted-foreground')}>
          <Highlight text={row.original.title ?? row.original.path} />
        </span>
      ),
    }),
    helper.accessor((item) => kindLabel(item.kind), {
      id: 'kind',
      header: 'Kind',
      sortFn: 'alphanumeric',
    }),
    helper.accessor((item) => text(item.category), {
      id: 'status',
      header: 'Status',
      sortFn: 'alphanumeric',
      sortUndefined: 'last',
      cell: ({ row }) => (
        <StatusBadge
          status={row.original.status}
          category={row.original.category}
          label={statusLabel(nodes, row.original.node, row.original.status)}
        />
      ),
    }),
    ranked('priority', 'Priority'),
    ranked('size', 'Size'),
    ranked('impact', 'Impact'),
    helper.accessor((item) => text(item.created), {
      id: 'created',
      header: 'Created',
      sortFn: 'alphanumeric',
      sortUndefined: 'last',
      cell: ({ row }) => <span className="font-mono text-xs whitespace-nowrap">{row.original.created ?? '—'}</span>,
    }),
  ])
}

export function ItemsTable({ items }: { items: Item[] }) {
  const { data: schema } = useSchema()
  const { data: nodes = [] } = useNodes()
  const columns = useMemo(() => buildColumns(schema, nodes), [schema, nodes])
  const [selected, setSelected] = useState<Item | null>(null)
  const table = useTable({ features, columns, data: items, getRowId: (item) => item.path })

  return (
    <>
      <div className="overflow-auto rounded-lg border border-border">
        <table className="w-full border-collapse text-sm">
          <thead className="sticky top-0 bg-muted/60 backdrop-blur">
            {table.getHeaderGroups().map((group) => (
              <tr key={group.id}>
                {group.headers.map((header) => {
                  const sorted = header.column.getIsSorted()
                  const Icon = sorted === 'asc' ? ArrowUp : sorted === 'desc' ? ArrowDown : ArrowUpDown
                  return (
                    <th
                      key={header.id}
                      scope="col"
                      aria-sort={sorted === 'asc' ? 'ascending' : sorted === 'desc' ? 'descending' : 'none'}
                      className="px-3 py-2 text-left text-xs font-semibold tracking-wide text-muted-foreground uppercase"
                    >
                      <button
                        type="button"
                        className="inline-flex items-center gap-1 hover:text-foreground"
                        onClick={header.column.getToggleSortingHandler()}
                      >
                        <table.FlexRender header={header} />
                        <Icon className={cn('size-3', !sorted && 'opacity-40')} aria-hidden />
                      </button>
                    </th>
                  )
                })}
              </tr>
            ))}
          </thead>
          <tbody>
            {table.getRowModel().rows.map((row) => (
              <tr
                key={row.id}
                tabIndex={0}
                onClick={() => setSelected(row.original)}
                onKeyDown={(event) => {
                  if (event.key === 'Enter') {
                    setSelected(row.original)
                  }
                }}
                className="cursor-pointer border-t border-border hover:bg-muted/50 focus-visible:bg-muted focus-visible:outline-none"
              >
                {row.getAllCells().map((cell) => (
                  <td key={cell.id} className="px-3 py-2 align-top">
                    <table.FlexRender cell={cell} />
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
        {items.length === 0 && <p className="p-8 text-center text-sm text-muted-foreground">No item matches.</p>}
      </div>
      <ItemDetails item={selected} onClose={() => setSelected(null)} />
    </>
  )
}
