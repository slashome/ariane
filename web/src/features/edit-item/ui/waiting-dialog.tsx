import { useState } from 'react'
import type { Item, Person, WaitingReason } from '@/shared/api'
import { cn } from '@/shared/lib'
import { Button, Dialog } from '@/shared/ui'
import './edit-item.css'

export interface Waiting {
  waiting_for: string[]
  waiting_on: WaitingReason
}

const reasons: { value: WaitingReason; label: string; hint: string }[] = [
  { value: 'review', label: 'A review', hint: 'of the code or the work' },
  { value: 'validation', label: 'A validation', hint: 'of the result, by a product manager or a client' },
  { value: 'answer', label: 'An answer', hint: 'a reply, a decision, an information' },
  { value: 'other', label: 'Something else', hint: '' },
]

// Asks who an item waits for, and why, before it moves to a waiting status. The people come from its
// node's directory (`[people]` in node.toml).
export function WaitingDialog({
  item,
  people,
  onConfirm,
  onCancel,
}: {
  item: Item | null
  people: Record<string, Person>
  onConfirm: (waiting: Waiting) => void
  onCancel: () => void
}) {
  const [chosen, setChosen] = useState<string[]>(item?.waiting_for ?? [])
  const [reason, setReason] = useState<WaitingReason>(item?.waiting_on ?? 'answer')
  const handles = Object.keys(people).sort((left, right) => (people[left]?.name ?? left).localeCompare(people[right]?.name ?? right))
  const toggle = (handle: string) =>
    setChosen((current) => (current.includes(handle) ? current.filter((entry) => entry !== handle) : [...current, handle]))

  return (
    <Dialog
      open={item !== null}
      onOpenChange={(open) => !open && onCancel()}
      title="Waiting for whom?"
      description={item ? `${item.id ?? ''} ${item.title ?? item.path}` : undefined}
      className="waiting-dialog"
    >
      {handles.length === 0 ? (
        <p className="waiting-dialog__empty">
          This node declares no one to wait for. Add a <code>[people.&lt;handle&gt;]</code> table with a <code>name</code> to its{' '}
          <code>node.toml</code>, or to a parent's.
        </p>
      ) : (
        <form
          className="waiting-dialog__form"
          onSubmit={(event) => {
            event.preventDefault()
            if (chosen.length > 0) {
              onConfirm({ waiting_for: chosen, waiting_on: reason })
            }
          }}
        >
          <fieldset className="waiting-dialog__fieldset">
            <legend className="waiting-dialog__legend">People</legend>
            <ul className="waiting-dialog__people">
              {handles.map((handle) => (
                <li key={handle}>
                  <label className="waiting-dialog__person">
                    <input type="checkbox" checked={chosen.includes(handle)} onChange={() => toggle(handle)} />
                    {people[handle]?.name ?? handle}
                    <span className="waiting-dialog__handle">{handle}</span>
                  </label>
                </li>
              ))}
            </ul>
          </fieldset>
          <fieldset className="waiting-dialog__fieldset">
            <legend className="waiting-dialog__legend">For</legend>
            <div className="waiting-dialog__reasons">
              {reasons.map((entry) => (
                <label
                  key={entry.value}
                  title={entry.hint}
                  className={cn('waiting-dialog__reason', reason === entry.value && 'waiting-dialog__reason--chosen')}
                >
                  <input
                    type="radio"
                    name="waiting-on"
                    className="lu-visually-hidden"
                    checked={reason === entry.value}
                    onChange={() => setReason(entry.value)}
                  />
                  {entry.label}
                </label>
              ))}
            </div>
          </fieldset>
          <div className="waiting-dialog__actions">
            <Button variant="ghost" onClick={onCancel}>
              Cancel
            </Button>
            <Button type="submit" disabled={chosen.length === 0}>
              Wait
            </Button>
          </div>
        </form>
      )}
    </Dialog>
  )
}
