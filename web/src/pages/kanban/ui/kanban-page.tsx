import { useMemo, useState } from 'react'
import { changesFor, ItemCard, useItems, useUpdateItem } from '@/entities/item'
import { useNodes } from '@/entities/node'
import { useSchema } from '@/entities/schema'
import { emptyFilters, filterItems, useItemFilters } from '@/features/filter-items'
import { displayNode } from '@/shared/lib'
import { DragArea, toast, type DragEndEvent } from '@/shared/ui'
import { ItemDetails } from '@/widgets/item-details'
import { buildColumns, targetStatus } from '../model/columns'
import { KanbanColumn } from './kanban-column'

const collapsedByDefault = new Set(['done', 'dropped'])

export function KanbanPage() {
  const { data: items = [], isPending } = useItems()
  const { data: nodes = [] } = useNodes()
  const { data: schema } = useSchema()
  const { filters } = useItemFilters()
  const update = useUpdateItem()
  const [toggled, setToggled] = useState<Set<string>>(() => new Set())
  const [opened, setOpened] = useState<string | null>(null)

  const scoped = useMemo(
    () =>
      filterItems(items, {
        ...emptyFilters,
        query: filters.query,
        node: filters.node,
        includeArchived: filters.includeArchived,
        kinds: ['task'],
      }),
    [items, filters.query, filters.node, filters.includeArchived],
  )
  const columns = useMemo(() => buildColumns(scoped, nodes, filters.node, schema), [scoped, nodes, filters.node, schema])
  const openedItem = items.find((item) => item.path === opened) ?? null

  const isCollapsed = (id: string, category: string) => collapsedByDefault.has(category) !== toggled.has(id)
  const toggle = (id: string) =>
    setToggled((current) => {
      const next = new Set(current)
      if (next.has(id)) {
        next.delete(id)
      } else {
        next.add(id)
      }
      return next
    })

  const onDragEnd = ({ active, over }: DragEndEvent) => {
    const item = scoped.find((entry) => entry.path === active.id)
    const column = columns.find((entry) => entry.id === over?.id)
    if (!item || !column) {
      return
    }
    const target = targetStatus(item, column, nodes)
    if (!target.ok) {
      toast.error(target.reason)
      return
    }
    if (target.status === item.status) {
      return
    }
    update.mutate({
      item,
      changes: changesFor(item, { status: target.status }),
      optimistic: { status: target.status, category: target.category },
    })
  }

  return (
    <div className="flex h-full flex-col gap-4 p-6">
      <div>
        <h1 className="text-xl font-semibold">Kanban</h1>
        <p className="text-sm text-muted-foreground">
          {isPending
            ? 'Loading…'
            : filters.node === null
              ? 'All nodes — one column per status category'
              : `${displayNode(filters.node)} — its workflow`}
        </p>
      </div>
      <DragArea
        onDragEnd={onDragEnd}
        renderOverlay={(id) => {
          const item = scoped.find((entry) => entry.path === id)
          return item ? <ItemCard item={item} showNode={item.node !== filters.node} className="shadow-lg" /> : null
        }}
      >
        <div className="flex min-h-0 flex-1 gap-3 overflow-x-auto pb-2">
          {columns.map((column) => (
            <KanbanColumn
              key={column.id}
              column={column}
              collapsed={isCollapsed(column.id, column.category)}
              scope={filters.node}
              onToggle={() => toggle(column.id)}
              onOpen={(item) => setOpened(item.path)}
            />
          ))}
        </div>
      </DragArea>
      <ItemDetails item={openedItem} onClose={() => setOpened(null)} />
    </div>
  )
}
