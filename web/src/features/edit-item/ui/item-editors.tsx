import { useState, type ReactNode } from 'react'
import { changesFor, today, useUpdateItem } from '@/entities/item'
import { peopleOf, useNodes, workflowOf } from '@/entities/node'
import { enumValues, useSchema } from '@/entities/schema'
import type { Item } from '@/shared/api'
import { NativeSelect, Segmented } from '@/shared/ui'
import { WaitingDialog, type Waiting } from './waiting-dialog'
import './edit-item.css'

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="item-editors__row">
      <span className="item-editors__label">{label}</span>
      <div className="item-editors__value">{children}</div>
    </div>
  )
}

// The fields of an item's header the interface edits — status, priority, size, impact — each saved
// when changed. A status of the waiting category first asks for whom; a ressource has no status.
export function ItemEditors({ item }: { item: Item }) {
  const { data: nodes = [] } = useNodes()
  const { data: schema } = useSchema()
  const update = useUpdateItem()
  const [waitingFor, setWaitingFor] = useState<string | null>(null)

  const statuses =
    item.kind === 'reflexion'
      ? (schema?.reflexion_statuses ?? []).map((id) => ({ id, label: id, category: null }))
      : item.kind === 'ressource'
        ? []
        : workflowOf(nodes, item.node)

  const setStatus = (status: string) => {
    const chosen = statuses.find((entry) => entry.id === status)
    if (chosen?.category === 'waiting') {
      setWaitingFor(status)
      return
    }
    update.mutate({
      item,
      changes: changesFor(item, { status }),
      optimistic: {
        status,
        category: chosen?.category ?? item.category,
        ...(item.category === 'waiting' ? { waiting_for: [], waiting_on: null, waiting_since: null } : {}),
      },
    })
  }

  const wait = ({ waiting_for, waiting_on }: Waiting) => {
    if (waitingFor === null) {
      return
    }
    const since = item.waiting_since ?? today()
    update.mutate({
      item,
      changes: changesFor(item, { status: waitingFor, waiting_for, waiting_on, waiting_since: since }),
      optimistic: { status: waitingFor, category: 'waiting', waiting_for, waiting_on, waiting_since: since },
    })
    setWaitingFor(null)
  }

  const setField = (field: 'priority' | 'size' | 'impact', value: string | null) =>
    update.mutate({ item, changes: changesFor(item, { [field]: value }), optimistic: { [field]: value } })

  return (
    <div className="item-editors">
      {statuses.length > 0 && (
        <Row label="Status">
          <NativeSelect aria-label="Status" className="item-editors__status" value={item.status ?? ''} onChange={(event) => setStatus(event.target.value)}>
            {item.status === null && <option value="">—</option>}
            {statuses.map((status) => (
              <option key={status.id} value={status.id}>
                {status.label}
              </option>
            ))}
          </NativeSelect>
        </Row>
      )}
      {item.kind !== 'ressource' && (
        <>
          <Row label="Priority">
            <Segmented label="Priority" values={enumValues(schema, 'priority')} value={item.priority} onChange={(value) => setField('priority', value)} />
          </Row>
          <Row label="Size">
            <Segmented label="Size" values={enumValues(schema, 'size')} value={item.size} onChange={(value) => setField('size', value)} />
          </Row>
          <Row label="Impact">
            <Segmented label="Impact" values={enumValues(schema, 'impact')} value={item.impact} onChange={(value) => setField('impact', value)} />
          </Row>
        </>
      )}
      <WaitingDialog
        key={waitingFor ?? 'none'}
        item={waitingFor === null ? null : item}
        people={peopleOf(nodes, item.node)}
        onConfirm={wait}
        onCancel={() => setWaitingFor(null)}
      />
    </div>
  )
}
