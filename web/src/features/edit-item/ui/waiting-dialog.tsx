import { useState } from 'react'
import type { Item, Person, WaitingReason } from '@/shared/api'
import { cn } from '@/shared/lib'
import { Button, Dialog } from '@/shared/ui'

export interface Waiting {
  waiting_for: string[]
  waiting_on: WaitingReason
}

const reasons: { value: WaitingReason; label: string; hint: string }[] = [
  { value: 'review', label: 'A review', hint: 'of the code or the work' },
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
      className="w-[30rem]"
    >
      {handles.length === 0 ? (
        <p className="text-sm text-muted-foreground">
          This node declares no one to wait for. Add a <code>[people.&lt;handle&gt;]</code> table with a <code>name</code> to its{' '}
          <code>node.toml</code>, or to a parent's.
        </p>
      ) : (
        <form
          className="flex flex-col gap-4"
          onSubmit={(event) => {
            event.preventDefault()
            if (chosen.length > 0) {
              onConfirm({ waiting_for: chosen, waiting_on: reason })
            }
          }}
        >
          <fieldset className="space-y-1">
            <legend className="mb-1 text-sm font-medium">People</legend>
            <ul className="max-h-56 space-y-0.5 overflow-y-auto">
              {handles.map((handle) => (
                <li key={handle}>
                  <label className="flex cursor-pointer items-center gap-2 rounded px-1.5 py-1 text-sm hover:bg-muted">
                    <input type="checkbox" checked={chosen.includes(handle)} onChange={() => toggle(handle)} />
                    {people[handle]?.name ?? handle}
                    <span className="font-mono text-[10px] text-muted-foreground">{handle}</span>
                  </label>
                </li>
              ))}
            </ul>
          </fieldset>
          <fieldset className="space-y-1">
            <legend className="mb-1 text-sm font-medium">For</legend>
            <div className="flex flex-wrap gap-2">
              {reasons.map((entry) => (
                <label
                  key={entry.value}
                  title={entry.hint}
                  className={cn(
                    'flex cursor-pointer items-center gap-1.5 rounded-full border border-border px-3 py-1 text-sm',
                    reason === entry.value && 'border-primary bg-primary/10',
                  )}
                >
                  <input
                    type="radio"
                    name="waiting-on"
                    className="sr-only"
                    checked={reason === entry.value}
                    onChange={() => setReason(entry.value)}
                  />
                  {entry.label}
                </label>
              ))}
            </div>
          </fieldset>
          <div className="flex justify-end gap-2">
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
