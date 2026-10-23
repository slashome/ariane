import { Lightbulb, NotebookPen } from 'lucide-react'
import { useMemo } from 'react'
import { Link, useLocation } from 'react-router'
import { changesFor, useUpdateItem } from '@/entities/item'
import { useSchema } from '@/entities/schema'
import { QueryField } from '@/features/filter-items'
import type { Item } from '@/shared/api'
import { routes, scopeSearch } from '@/shared/config'
import { DragArea, Draggable, DropZone, EmptyState, ErrorState, LoadingState, PageHeader, type DragEndEvent } from '@/shared/ui'
import { buildLanes } from '../model/reflexions'
import { ReflexionCard } from './reflexion-card'
import { useReflexions } from './use-reflexions'

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
    <div className="flex h-full flex-col gap-4 p-6">
      <PageHeader
        title="Reflexions"
        description={isPending ? 'Loading…' : `${reflexions.length} reflexions — drag a card, or use its menu, to change where it stands`}
        actions={
          <>
            <QueryField count={reflexions.length} total={total} />
            <Link to={{ pathname: routes.journal, search: scopeSearch(search) }} className="inline-flex items-center gap-1.5 text-sm text-primary hover:underline">
              <NotebookPen className="size-4" aria-hidden />
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
        <EmptyState icon={Lightbulb} title={total === 0 ? 'No reflexion here' : 'No reflexion matches the search'}>
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
          renderOverlay={(id) => {
            const item = reflexions.find((entry) => entry.path === id)
            return item ? <ReflexionCard item={item} scope={scope} className="w-80 shadow-lg" /> : null
          }}
        >
          <div className="grid min-h-0 flex-1 gap-3 lg:grid-cols-3">
            {lanes.map((lane) => (
              <DropZone key={lane.status} id={`lane:${lane.status}`} label={lane.label} className="flex min-h-40 flex-col gap-2 rounded-lg bg-muted/50 p-2">
                <div className="flex items-baseline justify-between px-1">
                  <h2 className="text-sm font-semibold">
                    {lane.label} <span className="font-normal text-muted-foreground">{lane.items.length}</span>
                  </h2>
                  <span className="text-xs text-muted-foreground">{lane.hint}</span>
                </div>
                <div className="flex flex-col gap-2 overflow-y-auto">
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
                            className="rounded border border-border bg-background px-1 py-0.5 text-[11px] text-muted-foreground"
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
                  {lane.items.length === 0 && <p className="px-1 py-4 text-center text-xs text-muted-foreground">Nothing {lane.label.toLowerCase()}</p>}
                </div>
              </DropZone>
            ))}
          </div>
        </DragArea>
      )}
    </div>
  )
}
