import { useState } from 'react'
import { useAddPersona, useChangePersona } from '../api/use-personas'
import { isPersonaId } from '../model/handle'
import type { PersonaEntry } from '@/shared/api'
import { Button, Dialog, Input } from '@/shared/ui'

// Adds a persona to a node, or edits one: name, and id (renamed in every story written for it).
export function PersonaDialog({
  node,
  persona,
  onClose,
}: {
  node: string
  // The persona to edit; `null` to add one.
  persona: PersonaEntry | null
  onClose: () => void
}) {
  const add = useAddPersona()
  const change = useChangePersona()
  const [name, setName] = useState(persona?.name ?? '')
  const [id, setId] = useState(persona?.id ?? '')
  const valid = name.trim() !== '' && isPersonaId(id)

  const save = () => {
    if (!valid) {
      return
    }
    const done = { onSuccess: onClose }
    if (persona === null) {
      add.mutate({ node, id, name: name.trim() }, done)
    } else {
      change.mutate({ node, id: persona.id, name: name.trim(), rename_to: id !== persona.id ? id : undefined }, done)
    }
  }

  return (
    <Dialog
      open
      onOpenChange={(open) => !open && onClose()}
      title={persona === null ? 'Add a persona' : `Edit ${persona.name}`}
      description={`Declared in ${node === '' ? '(home)' : node}, for every story below it.`}
      className="w-[32rem]"
    >
      <form
        className="flex flex-col gap-4"
        onSubmit={(event) => {
          event.preventDefault()
          save()
        }}
      >
        <label className="space-y-1 text-sm">
          <span className="font-medium">Name</span>
          <Input value={name} onChange={(event) => setName(event.target.value)} autoFocus />
        </label>
        <label className="space-y-1 text-sm">
          <span className="font-medium">Id</span>
          <Input
            value={id}
            className="font-mono"
            placeholder="group:role"
            aria-invalid={id !== '' && !isPersonaId(id)}
            onChange={(event) => setId(event.target.value)}
          />
          <span className="block text-xs text-muted-foreground">
            What stories write in <code>personas</code>, such as <code>user:rh</code>.
            {persona !== null && id !== persona.id && ` Renaming rewrites the ${persona.used_by.length} stories written for it.`}
          </span>
        </label>
        <div className="flex justify-end gap-2">
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button type="submit" disabled={!valid || add.isPending || change.isPending}>
            {persona === null ? 'Add' : 'Save'}
          </Button>
        </div>
      </form>
    </Dialog>
  )
}
