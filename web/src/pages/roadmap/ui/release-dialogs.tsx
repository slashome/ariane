import { useState, type FormEvent, type ReactNode } from 'react'
import type { Release, ReleaseChange } from '@/shared/api'
import { Button, Dialog, Input, NativeSelect } from '@/shared/ui'
import { lastPlace, placeChoices } from '../model/column-menu'

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="flex flex-col gap-1 text-sm">
      <span className="text-xs text-muted-foreground">{label}</span>
      {children}
    </label>
  )
}

export function NewReleaseDialog({
  open,
  node,
  shownOrder,
  pending,
  onClose,
  onCreate,
}: {
  open: boolean
  node: string
  shownOrder: Release[]
  pending: boolean
  onClose: () => void
  onCreate: (change: ReleaseChange) => void
}) {
  const [name, setName] = useState('')
  const [place, setPlace] = useState(lastPlace)
  const [target, setTarget] = useState('')
  const choices = placeChoices(shownOrder)
  const submit = (event: FormEvent) => {
    event.preventDefault()
    const choice = choices.find((entry) => entry.value === place) ?? choices[0]
    onCreate({ node, name: name.trim(), ...choice?.change, ...(target === '' ? {} : { target }) })
  }
  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()} title="New release" className="w-[26rem]">
      <form onSubmit={submit} className="flex flex-col gap-3">
        <Field label="Name">
          <Input autoFocus value={name} onChange={(event) => setName(event.target.value)} placeholder="0.4, mango…" />
        </Field>
        <Field label="Position">
          <NativeSelect aria-label="Position" value={place} onChange={(event) => setPlace(event.target.value)}>
            {choices.map((choice) => (
              <option key={choice.value} value={choice.value}>
                {choice.label}
              </option>
            ))}
          </NativeSelect>
        </Field>
        <Field label="Target date (optional)">
          <Input type="date" aria-label="Target date" value={target} onChange={(event) => setTarget(event.target.value)} />
        </Field>
        <div className="flex justify-end gap-2">
          <Button variant="outline" size="sm" onClick={onClose}>
            Cancel
          </Button>
          <Button type="submit" size="sm" disabled={pending || name.trim() === ''}>
            Create
          </Button>
        </div>
      </form>
    </Dialog>
  )
}

export function RenameReleaseDialog({
  release,
  pending,
  onClose,
  onRename,
}: {
  release: Release | null
  pending: boolean
  onClose: () => void
  onRename: (name: string) => void
}) {
  const [name, setName] = useState(release?.name ?? '')
  const submit = (event: FormEvent) => {
    event.preventDefault()
    onRename(name.trim())
  }
  return (
    <Dialog
      open={release !== null}
      onOpenChange={(next) => !next && onClose()}
      title={`Rename ${release?.name ?? ''}`}
      description="Every item planned in it is rewritten."
      className="w-[26rem]"
    >
      <form onSubmit={submit} className="flex flex-col gap-3">
        <Field label="New name">
          <Input autoFocus value={name} onChange={(event) => setName(event.target.value)} />
        </Field>
        <div className="flex justify-end gap-2">
          <Button variant="outline" size="sm" onClick={onClose}>
            Cancel
          </Button>
          <Button type="submit" size="sm" disabled={pending || name.trim() === '' || name.trim() === release?.name}>
            Rename
          </Button>
        </div>
      </form>
    </Dialog>
  )
}
