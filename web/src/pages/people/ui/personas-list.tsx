import { Pencil, Plus, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { useRemovePersona } from '../api/use-personas'
import { ApiError, type PersonaDirectory, type PersonaEntry } from '@/shared/api'
import { displayNode } from '@/shared/lib'
import { Button, ConfirmDialog, DataTable, EmptyState, Highlight, type Column } from '@/shared/ui'
import { References } from './item-reference'
import { PersonaDialog } from './persona-dialog'

type Editing = { node: string; persona: PersonaEntry | null }

// The personas of each node, the stories written for each, and their add, edit and remove dialogs.
export function PersonasList({ directories, searching }: { directories: PersonaDirectory[]; searching: boolean }) {
  const remove = useRemovePersona()
  const [editing, setEditing] = useState<Editing | null>(null)
  const [removing, setRemoving] = useState<{ node: string; persona: PersonaEntry } | null>(null)
  const [blocked, setBlocked] = useState<string[] | null>(null)

  const confirmRemove = () => {
    if (removing === null) {
      return
    }
    remove.mutate(
      { node: removing.node, id: removing.persona.id },
      {
        onSettled: () => setRemoving(null),
        onError: (failure) => {
          if (failure instanceof ApiError && failure.status === 409) {
            const items = (failure.body as { items?: string[] } | null)?.items ?? []
            setBlocked(items)
          }
        },
      },
    )
  }

  const columnsFor = (node: string): Column<PersonaEntry>[] => [
    {
      id: 'name',
      header: 'Name',
      sortValue: (persona) => persona.name,
      className: 'font-medium',
      cell: (persona) => <Highlight text={persona.name} />,
    },
    {
      id: 'id',
      header: 'Id',
      sortValue: (persona) => persona.id,
      className: 'font-mono text-xs',
      cell: (persona) => <Highlight text={persona.id} />,
    },
    {
      id: 'stories',
      header: 'Stories',
      sortValue: (persona) => persona.used_by.length || null,
      cell: (persona) => <References references={persona.used_by} />,
    },
    {
      id: 'actions',
      header: '',
      headerClassName: 'w-20',
      cell: (persona) => (
        <div className="flex justify-end gap-1">
          <button type="button" className="lu-icon-button size-7" aria-label={`Edit ${persona.name}`} onClick={() => setEditing({ node, persona })}>
            <Pencil className="size-3.5" aria-hidden />
          </button>
          <button
            type="button"
            className="lu-icon-button size-7 hover:text-red-600"
            aria-label={`Remove ${persona.name}`}
            onClick={() => setRemoving({ node, persona })}
          >
            <Trash2 className="size-3.5" aria-hidden />
          </button>
        </div>
      ),
    },
  ]

  if (directories.length === 0) {
    return (
      <EmptyState icon="user" title="No tracker here">
        Personas are declared in a node that binds a tracker or already declares some (<code>[personas]</code> in its <code>node.toml</code>).
      </EmptyState>
    )
  }

  return (
    <>
      {directories.map((directory) => (
        <section key={directory.node} className="space-y-2">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <h2 className="text-sm font-semibold">
              <span className="font-mono">{displayNode(directory.node)}</span>
            </h2>
            <Button size="sm" variant="outline" onClick={() => setEditing({ node: directory.node, persona: null })}>
              <Plus className="size-3.5" aria-hidden />
              Add a persona
            </Button>
          </div>
          {directory.personas.length === 0 ? (
            <p className="rounded-lg border border-dashed border-border p-4 text-sm text-muted-foreground">
              {searching ? 'No persona here of a story the search finds.' : 'No persona yet.'}
            </p>
          ) : (
            <DataTable
              rows={directory.personas}
              columns={columnsFor(directory.node)}
              rowKey={(persona) => persona.id}
              storageKey="personas"
              label={`Personas of ${displayNode(directory.node)}`}
            />
          )}
        </section>
      ))}
      {editing && (
        <PersonaDialog
          key={`${editing.node}:${editing.persona?.id ?? 'new'}`}
          node={editing.node}
          persona={editing.persona}
          onClose={() => setEditing(null)}
        />
      )}
      <ConfirmDialog
        open={removing !== null}
        onOpenChange={(open) => !open && setRemoving(null)}
        title={`Remove ${removing?.persona.name ?? ''}?`}
        description="It leaves the personas of this node. Refused while stories are still written for it."
        confirmLabel={remove.isPending ? 'Removing…' : 'Remove'}
        pending={remove.isPending}
        onConfirm={confirmRemove}
      />
      <ConfirmDialog
        open={blocked !== null}
        onOpenChange={(open) => !open && setBlocked(null)}
        title="Still in use"
        description={`These stories are written for this persona: ${(blocked ?? []).join(', ')}. Take it out of their personas first.`}
        confirmLabel="OK"
        onConfirm={() => setBlocked(null)}
      />
    </>
  )
}
