import { useMemo, useState } from 'react'
import { changesFor, formatHours, ItemCard, useItems, useUpdateItem } from '@/entities/item'
import { useNodes } from '@/entities/node'
import { useSchema } from '@/entities/schema'
import { QueryField, useFilteredItems } from '@/features/filter-items'
import { DragArea, Draggable, Switch, type DragEndEvent } from '@/shared/ui'
import { parseCellId, placeItems, sharedPricing, sizeHours } from '../model/placement'
import { ImpactRow } from './impact-row'

export function MatrixPage() {
  const { data: items = [], isPending } = useItems()
  const { data: schema } = useSchema()
  const { data: nodes = [] } = useNodes()
  const update = useUpdateItem()
  const [includeClosed, setIncludeClosed] = useState(false)

  const { items: scoped, total } = useFilteredItems(items, { kinds: ['task'] })
  const matrix = useMemo(() => placeItems(scoped, schema, includeClosed), [scoped, schema, includeClosed])
  const pricing = useMemo(() => sharedPricing(nodes, scoped), [nodes, scoped])

  const onDragEnd = ({ active, over }: DragEndEvent) => {
    const item = scoped.find((entry) => entry.path === active.id)
    const cell = over ? parseCellId(String(over.id)) : null
    if (!item || !cell) {
      return
    }
    const changes = changesFor(item, { size: cell.size, impact: cell.impact })
    if (Object.keys(changes).length === 0) {
      return
    }
    update.mutate({ item, changes, optimistic: { size: cell.size, impact: cell.impact } })
  }

  return (
    <div className="flex flex-col gap-4 p-6">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h1 className="text-xl font-semibold">Size × impact</h1>
          <p className="text-sm text-muted-foreground">
            {isPending ? 'Loading…' : 'Drag an item to estimate it. Quick wins are small and high-impact.'}
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <label className="flex items-center gap-2 text-sm">
            <Switch checked={includeClosed} onCheckedChange={setIncludeClosed} aria-label="Include done and dropped" />
            Include done and dropped
          </label>
          <QueryField count={scoped.length} total={total} />
        </div>
      </div>
      <DragArea
        onDragEnd={onDragEnd}
        renderOverlay={(id) => {
          const item = scoped.find((entry) => entry.path === id)
          return item ? <ItemCard item={item} showNode className="shadow-lg" /> : null
        }}
      >
        <div
          className="grid gap-2"
          style={{ gridTemplateColumns: `6rem repeat(${matrix.sizes.length}, minmax(11rem, 1fr))` }}
        >
          <div />
          {matrix.sizes.map((size) => {
            const hours = sizeHours(pricing, size)
            return (
              <div key={size} className="text-center text-xs font-semibold tracking-wide text-muted-foreground uppercase">
                {size}
                {hours !== null && <span className="ml-1.5 font-mono font-normal normal-case">{formatHours(hours)}</span>}
              </div>
            )
          })}
          {matrix.impacts.map((impact) => (
            <ImpactRow key={impact} impact={impact} matrix={matrix} />
          ))}
        </div>
        <section className="rounded-lg border border-dashed border-border p-3">
          <h2 className="mb-2 text-sm font-medium">
            Unestimated <span className="text-muted-foreground">({matrix.unestimated.length})</span>
          </h2>
          <div className="grid grid-cols-[repeat(auto-fill,minmax(14rem,1fr))] gap-2">
            {matrix.unestimated.map((item) => (
              <Draggable key={item.path} id={item.path}>
                <ItemCard item={item} showNode />
              </Draggable>
            ))}
          </div>
        </section>
      </DragArea>
    </div>
  )
}
