import { Pencil, Plus, Trash2, UserRound } from 'lucide-react'
import { useState } from 'react'
import { useRemovePersona } from '../api/use-personas'
import { ApiError, type PersonaDirectory, type PersonaEntry } from '@/shared/api'
import { displayNode } from '@/shared/lib'
import { Button, ConfirmDialog, EmptyState, Highlight } from '@/shared/ui'
import { ItemReference } from './item-reference'
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

  if (directories.length === 0) {
    return (
      <EmptyState icon={UserRound} title="No tracker here">
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
            <table className="w-full overflow-hidden rounded-lg border border-border text-sm">
              <thead className="bg-muted/50 text-left text-xs text-muted-foreground">
                <tr>
                  <th className="px-3 py-2 font-medium">Name</th>
                  <th className="px-3 py-2 font-medium">Id</th>
                  <th className="px-3 py-2 font-medium">Stories</th>
                  <th className="w-20 px-3 py-2" />
                </tr>
              </thead>
              <tbody>
                {directory.personas.map((persona) => (
                  <tr key={persona.id} className="border-t border-border align-top">
                    <td className="px-3 py-2 font-medium">
                      <Highlight text={persona.name} />
                    </td>
                    <td className="px-3 py-2 font-mono text-xs">
                      <Highlight text={persona.id} />
                    </td>
                    <td className="px-3 py-2">
                      {persona.used_by.length === 0 ? (
                        <span className="text-xs text-muted-foreground">—</span>
                      ) : (
                        <ul className="flex flex-wrap gap-1">
                          {persona.used_by.map((reference) => (
                            <li key={reference}>
                              <ItemReference reference={reference} />
                            </li>
                          ))}
                        </ul>
                      )}
                    </td>
                    <td className="px-3 py-2">
                      <div className="flex justify-end gap-1">
                        <button
                          type="button"
                          className="icon-button size-7"
                          aria-label={`Edit ${persona.name}`}
                          onClick={() => setEditing({ node: directory.node, persona })}
                        >
                          <Pencil className="size-3.5" aria-hidden />
                        </button>
                        <button
                          type="button"
                          className="icon-button size-7 hover:text-red-600"
                          aria-label={`Remove ${persona.name}`}
                          onClick={() => setRemoving({ node: directory.node, persona })}
                        >
                          <Trash2 className="size-3.5" aria-hidden />
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
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
