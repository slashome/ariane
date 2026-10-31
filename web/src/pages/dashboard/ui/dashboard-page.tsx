import { closestCenter, DndContext, KeyboardSensor, PointerSensor, useSensor, useSensors, type DragEndEvent } from '@dnd-kit/core'
import { arrayMove, rectSortingStrategy, SortableContext, sortableKeyboardCoordinates } from '@dnd-kit/sortable'
import { useMemo, useState } from 'react'
import { useItems } from '@/entities/item'
import { findNode, sortNodes, useDashboard, useNodes, useSaveDashboard } from '@/entities/node'
import { useSchema } from '@/entities/schema'
import type { Category } from '@/shared/api'
import { displayNode } from '@/shared/lib'
import { Button, Icon, NativeSelect } from '@/shared/ui'
import { cardStats } from '../model/card-stats'
import { NodeCard } from './node-card'
import { SortableCard } from './sortable-card'
import './dashboard.css'

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
    <div className="dashboard">
      <div className="dashboard__header">
        <div>
          <h1 className="dashboard__title">Dashboard</h1>
          <p className="dashboard__summary">
            {isPending ? 'Loading…' : `${pinned.length} pinned ${pinned.length === 1 ? 'node' : 'nodes'}`}
          </p>
        </div>
        {editing ? (
          <div className="dashboard__actions">
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
              <Icon name="add" className="dashboard__icon" />
              Pin
            </Button>
            <Button variant="ghost" onClick={() => setDraft(null)}>
              Cancel
            </Button>
            <Button onClick={finish}>Save</Button>
          </div>
        ) : (
          <Button variant="outline" onClick={() => setDraft(dashboard?.pinned ?? [])}>
            <Icon name="edit" className="dashboard__icon" />
            Edit
          </Button>
        )}
      </div>

      {pinned.length === 0 ? (
        <div className="dashboard__empty">
          <p className="dashboard__empty-title">No pinned node yet</p>
          <p>Click Edit, pick a node and Pin it: its card shows its progress, quick wins and blockers.</p>
        </div>
      ) : editing ? (
        <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={onDragEnd}>
          <SortableContext items={pinned} strategy={rectSortingStrategy}>
            <div className="dashboard__grid">
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
                      className="node-card--editing"
                    />
                  )}
                />
              ))}
            </div>
          </SortableContext>
        </DndContext>
      ) : (
        <div className="dashboard__grid">
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
