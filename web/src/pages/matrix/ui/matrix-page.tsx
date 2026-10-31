import { useMemo, useState } from 'react'
import { changesFor, ItemCard, useItems, useUpdateItem } from '@/entities/item'
import { useSchema } from '@/entities/schema'
import { QueryField, useFilteredItems } from '@/features/filter-items'
import { DragArea, Draggable, Switch, type DragEndEvent } from '@/shared/ui'
import { parseCellId, placeItems } from '../model/placement'
import { ImpactRow } from './impact-row'
import './matrix.css'

export function MatrixPage() {
  const { data: items = [], isPending } = useItems()
  const { data: schema } = useSchema()
  const update = useUpdateItem()
  const [includeClosed, setIncludeClosed] = useState(false)

  const { items: scoped, total } = useFilteredItems(items, { kinds: ['task'] })
  const matrix = useMemo(() => placeItems(scoped, schema, includeClosed), [scoped, schema, includeClosed])

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
    <div className="matrix-page">
      <div className="matrix-page__header">
        <div>
          <h1 className="matrix-page__title">Size × impact</h1>
          <p className="matrix-page__description">
            {isPending ? 'Loading…' : 'Drag an item to estimate it. Quick wins are small and high-impact.'}
          </p>
        </div>
        <div className="matrix-page__actions">
          <label className="matrix-page__toggle">
            <Switch checked={includeClosed} onCheckedChange={setIncludeClosed} aria-label="Include done and dropped" />
            Include done and dropped
          </label>
          <QueryField count={scoped.length} total={total} />
        </div>
      </div>
      <DragArea
        onDragEnd={onDragEnd}
      >
        <div
          className="matrix-page__grid"
          style={{ gridTemplateColumns: `6rem repeat(${matrix.sizes.length}, minmax(11rem, 1fr))` }}
        >
          <div />
          {matrix.sizes.map((size) => (
            <div key={size} className="matrix-page__size">
              {size}
            </div>
          ))}
          {matrix.impacts.map((impact) => (
            <ImpactRow key={impact} impact={impact} matrix={matrix} />
          ))}
        </div>
        <section className="matrix-page__unestimated">
          <h2 className="matrix-page__unestimated-title">
            Unestimated <span className="matrix-page__count">({matrix.unestimated.length})</span>
          </h2>
          <div className="matrix-page__unestimated-items">
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
