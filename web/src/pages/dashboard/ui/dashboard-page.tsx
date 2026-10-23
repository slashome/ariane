import { closestCenter, DndContext, KeyboardSensor, PointerSensor, useSensor, useSensors, type DragEndEvent } from '@dnd-kit/core'
import { arrayMove, rectSortingStrategy, SortableContext, sortableKeyboardCoordinates } from '@dnd-kit/sortable'
import { Pencil, Plus } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useItems } from '@/entities/item'
import { findNode, sortNodes, useDashboard, useNodes, useSaveDashboard } from '@/entities/node'
import { useSchema } from '@/entities/schema'
import type { Category } from '@/shared/api'
import { displayNode } from '@/shared/lib'
import { Button, NativeSelect } from '@/shared/ui'
import { cardStats } from '../model/card-stats'
import { NodeCard } from './node-card'
import { SortableCard } from './sortable-card'

const noCandidate = 'none'

export function DashboardPage() {
  const { data: dashboard, isPending } = useDashboard()
  const { data: items = [] } = useItems()
  const { data: nodes = [] } = useNodes()
  const { data: schema } = useSchema()
  const save = useSaveDashboard()
  const [draft, setDraft] = useState<string[] | null>(null)
  const [candidate, setCandidate] = useState<string | null>(null)
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  )

  const pinned = useMemo(() => draft ?? dashboard?.pinned ?? [], [draft, dashboard])
  const editing = draft !== null
  const labels = (category: Category) => schema?.categories.find((entry) => entry.id === category)?.label ?? category
  const stats = useMemo(
    () => new Map(pinned.map((path) => [path, cardStats(items, nodes, path)])),
    [pinned, items, nodes],
  )
  const unpinned = sortNodes(nodes).filter((node) => !pinned.includes(node.path))

  const onDragEnd = ({ active, over }: DragEndEvent) => {
    if (!over || active.id === over.id) {
      return
    }
    setDraft((current) => {
      const list = current ?? []
      return arrayMove(list, list.indexOf(String(active.id)), list.indexOf(String(over.id)))
    })
  }

  const finish = () => {
    if (draft !== null) {
      save.mutate({ pinned: draft })
    }
    setDraft(null)
  }

  return (
    <div className="flex flex-col gap-4 p-6">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h1 className="text-xl font-semibold">Dashboard</h1>
          <p className="text-sm text-muted-foreground">
            {isPending ? 'Loading…' : `${pinned.length} pinned ${pinned.length === 1 ? 'node' : 'nodes'}`}
          </p>
        </div>
        {editing ? (
          <div className="flex items-center gap-2">
            <NativeSelect
              aria-label="Node to pin"
              value={candidate === null ? noCandidate : `node:${candidate}`}
              onChange={(event) =>
                setCandidate(event.target.value === noCandidate ? null : event.target.value.slice('node:'.length))
              }
            >
              <option value={noCandidate}>Pin a node…</option>
              {unpinned.map((node) => (
                <option key={node.path} value={`node:${node.path}`}>
                  {displayNode(node.path)}
                </option>
              ))}
            </NativeSelect>
            <Button
              variant="outline"
              disabled={candidate === null}
              onClick={() => {
                if (candidate !== null) {
                  setDraft([...pinned, candidate])
                  setCandidate(null)
                }
              }}
            >
              <Plus className="size-4" aria-hidden />
              Pin
            </Button>
            <Button variant="ghost" onClick={() => setDraft(null)}>
              Cancel
            </Button>
            <Button onClick={finish}>Save</Button>
          </div>
        ) : (
          <Button variant="outline" onClick={() => setDraft(dashboard?.pinned ?? [])}>
            <Pencil className="size-4" aria-hidden />
            Edit
          </Button>
        )}
      </div>

      {pinned.length === 0 ? (
        <div className="rounded-lg border border-dashed border-border p-10 text-center text-sm text-muted-foreground">
          <p className="font-medium text-foreground">No pinned node yet</p>
          <p>Click Edit, pick a node and Pin it: its card shows its progress, quick wins and blockers.</p>
        </div>
      ) : editing ? (
        <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={onDragEnd}>
          <SortableContext items={pinned} strategy={rectSortingStrategy}>
            <div className="grid grid-cols-[repeat(auto-fill,minmax(22rem,1fr))] gap-4">
              {pinned.map((path) => (
                <SortableCard
                  key={path}
                  id={path}
                  onRemove={() => setDraft(pinned.filter((entry) => entry !== path))}
                  render={(toolbar) => (
                    <NodeCard
                      path={path}
                      description={findNode(nodes, path)?.description}
                      stats={stats.get(path) ?? cardStats(items, nodes, path)}
                      labels={labels}
                      toolbar={toolbar}
                      className="ring-1 ring-primary/20"
                    />
                  )}
                />
              ))}
            </div>
          </SortableContext>
        </DndContext>
      ) : (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(22rem,1fr))] gap-4">
          {pinned.map((path) => (
            <NodeCard
              key={path}
              path={path}
              description={findNode(nodes, path)?.description}
              stats={stats.get(path) ?? cardStats(items, nodes, path)} labels={labels} />
          ))}
        </div>
      )}
    </div>
  )
}
