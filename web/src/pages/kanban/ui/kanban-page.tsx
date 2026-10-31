import { useMemo, useState } from 'react'
import { changesFor, dropBeside, ItemCard, today, useItems, useRankItem, useUpdateItem, waitingSummary } from '@/entities/item'
import { peopleOf, useNodes } from '@/entities/node'
import { useSchema } from '@/entities/schema'
import { QueryField, useFilteredItems, useItemFilters } from '@/features/filter-items'
import { WaitingDialog, type Waiting } from '@/features/edit-item'
import type { Item } from '@/shared/api'
import { displayNode, neighbours, useSearchParam, useStoredSet } from '@/shared/lib'
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
  const rank = useRankItem()
  // Columns folded or unfolded against their default, remembered by the browser.
  const [toggled, toggle] = useStoredSet('ariane.kanban.toggled')
  // The previewed item lives in the URL, so Back from its page brings the preview back.
  const [opened, setOpened] = useSearchParam('item')
  const [moving, setMoving] = useState<{ item: Item; status: string } | null>(null)

  const { items: scoped, total } = useFilteredItems(items, { kinds: ['task'] })
  const columns = useMemo(() => buildColumns(scoped, nodes, filters.node, schema), [scoped, nodes, filters.node, schema])
  const openedItem = items.find((item) => item.path === opened) ?? null

  const isCollapsed = (id: string, category: string) => collapsedByDefault.has(category) !== toggled.has(id)
  const onDragEnd = ({ active, over }: DragEndEvent) => {
    const item = scoped.find((entry) => entry.path === active.id)
    const drop = item && over ? dropBeside(item, String(over.id), columns) : null
    if (!item || !drop) {
      return
    }
    const target = targetStatus(item, drop.lane, nodes)
    if (!target.ok) {
      toast.error(target.reason)
      return
    }
    const moves = target.status !== item.status
    if (moves && target.category === 'waiting') {
      setMoving({ item, status: target.status })
      return
    }
    if (drop.place !== null && drop.other !== null) {
      rank.mutate({
        item,
        place: drop.place,
        other: drop.other,
        status: moves ? { to: target.status, category: target.category } : undefined,
      })
      return
    }
    if (!moves) {
      return
    }
    update.mutate({
      item,
      changes: changesFor(item, { status: target.status }),
      optimistic: {
        status: target.status,
        category: target.category,
        ...(item.category === 'waiting' ? { waiting_for: [], waiting_on: null, waiting_since: null } : {}),
      },
    })
  }

  const wait = ({ waiting_for, waiting_on }: Waiting) => {
    if (moving === null) {
      return
    }
    const { item, status } = moving
    const since = item.waiting_since ?? today()
    update.mutate({
      item,
      changes: changesFor(item, { status, waiting_for, waiting_on, waiting_since: since }),
      optimistic: { status, category: 'waiting', waiting_for, waiting_on, waiting_since: since },
    })
    setMoving(null)
  }

  const waitingOf = (item: Item) =>
    item.waiting_for.length > 0 ? waitingSummary(item, peopleOf(nodes, item.node)) : undefined

  // The order the preview steps through: the cards of the open columns, left to right, top to bottom.
  const visible = columns.filter((column) => !isCollapsed(column.id, column.category)).flatMap((column) => column.items)
  const around = opened === null ? null : neighbours(visible, (item) => item.path === opened)

  return (
    <div className="flex h-full flex-col gap-4 p-6">
      <div className="flex flex-wrap items-end justify-between gap-3">
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
        <QueryField count={scoped.length} total={total} />
      </div>
      <DragArea
        sortable
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
              selected={opened}
              waitingOf={waitingOf}
            />
          ))}
        </div>
      </DragArea>
      <ItemDetails
        item={openedItem}
        onClose={() => setOpened(null)}
        stepper={
          around === null
            ? undefined
            : {
                position: around.position,
                count: visible.length,
                onPrevious: around.previous ? () => setOpened(around.previous?.path ?? null) : undefined,
                onNext: around.next ? () => setOpened(around.next?.path ?? null) : undefined,
              }
        }
      />
      <WaitingDialog
        key={moving?.item.path ?? 'none'}
        item={moving?.item ?? null}
        people={moving ? peopleOf(nodes, moving.item.node) : {}}
        onConfirm={wait}
        onCancel={() => setMoving(null)}
      />
    </div>
  )
}
