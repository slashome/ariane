import { useMemo } from 'react'
import { Link, useLocation } from 'react-router'
import { changesFor, useUpdateItem } from '@/entities/item'
import { useSchema } from '@/entities/schema'
import { QueryField } from '@/features/filter-items'
import type { Item } from '@/shared/api'
import { routes, scopeSearch } from '@/shared/config'
import { DragArea, Draggable, DropZone, EmptyState, ErrorState, Icon, LoadingState, PageHeader, type DragEndEvent } from '@/shared/ui'
import { buildLanes } from '../model/reflexions'
import { ReflexionCard } from './reflexion-card'
import { useReflexions } from './use-reflexions'
import './reflexions.css'

export function ReflexionsPage() {
  const { reflexions, total, isPending, error, scope } = useReflexions()
  const { data: schema } = useSchema()
  const update = useUpdateItem()
  const { search } = useLocation()
  const lanes = useMemo(() => buildLanes(reflexions, schema?.reflexion_statuses ?? []), [reflexions, schema])

  const move = (item: Item, status: string) => {
    if (status !== item.status) {
      update.mutate({ item, changes: changesFor(item, { status }), optimistic: { status } })
    }
  }

  const onDragEnd = ({ active, over }: DragEndEvent) => {
    const item = reflexions.find((entry) => entry.path === active.id)
    const status = typeof over?.id === 'string' ? over.id.replace(/^lane:/, '') : null
    if (item && status) {
      move(item, status)
    }
  }

  return (
    <div className="reflexions">
      <PageHeader
        title="Reflexions"
        description={isPending ? 'Loading…' : `${reflexions.length} reflexions — drag a card, or use its menu, to change where it stands`}
        actions={
          <>
            <QueryField count={reflexions.length} total={total} />
            <Link to={{ pathname: routes.journal, search: scopeSearch(search) }} className="reflexions__link reflexions__link--icon">
              <Icon name="notebook" className="reflexions__link-icon" />
              Journal of settled reflexions
            </Link>
          </>
        }
      />
      {error ? (
        <ErrorState message={`Could not load the items: ${error.message}`} />
      ) : isPending ? (
        <LoadingState />
      ) : reflexions.length === 0 ? (
        <EmptyState icon="idea" title={total === 0 ? 'No reflexion here' : 'No reflexion matches the search'}>
          {total === 0 ? (
            <>
              A reflexion is a Markdown file under <code>reflexions/</code>, open until it is settled.
            </>
          ) : (
            `${total} reflexions in scope; the search field keeps none of them.`
          )}
        </EmptyState>
      ) : (
        <DragArea
          onDragEnd={onDragEnd}
        >
          <div className="reflexions__lanes">
            {lanes.map((lane) => (
              <DropZone key={lane.status} id={`lane:${lane.status}`} label={lane.label} className="reflexions__lane">
                <div className="reflexions__lane-header">
                  <h2 className="reflexions__lane-title">
                    {lane.label} <span className="reflexions__lane-count">{lane.items.length}</span>
                  </h2>
                  <span className="reflexions__lane-hint">{lane.hint}</span>
                </div>
                <div className="reflexions__lane-cards">
                  {lane.items.map((item) => (
                    <Draggable key={item.path} id={item.path}>
                      <ReflexionCard
                        item={item}
                        scope={scope}
                        action={
                          <select
                            aria-label={`Status of ${item.id ?? item.path}`}
                            value={item.status ?? ''}
                            onChange={(event) => move(item, event.target.value)}
                            onPointerDown={(event) => event.stopPropagation()}
                            onKeyDown={(event) => event.stopPropagation()}
                            className="reflexions__status"
                          >
                            {lanes.map((option) => (
                              <option key={option.status} value={option.status}>
                                {option.label}
                              </option>
                            ))}
                          </select>
                        }
                      />
                    </Draggable>
                  ))}
                  {lane.items.length === 0 && <p className="reflexions__lane-empty">Nothing {lane.label.toLowerCase()}</p>}
                </div>
              </DropZone>
            ))}
          </div>
        </DragArea>
      )}
    </div>
  )
}
