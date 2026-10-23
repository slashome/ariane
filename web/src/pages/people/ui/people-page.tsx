import { Pencil, Plus, Trash2, Users } from 'lucide-react'
import { useState } from 'react'
import { usePeople, useRemovePerson } from '../api/use-people'
import { useItemFilters } from '@/features/filter-items'
import { ApiError, type Directory, type PersonEntry } from '@/shared/api'
import { displayNode, isWithinNode } from '@/shared/lib'
import { Button, ConfirmDialog, EmptyState, ErrorState, LoadingState, PageHeader } from '@/shared/ui'
import { ItemReference } from './item-reference'
import { PersonDialog } from './person-dialog'

type Editing = { directory: Directory; person: PersonEntry | null }

export function PeoplePage() {
  const { data: directories = [], isPending, error } = usePeople()
  const { filters } = useItemFilters()
  const remove = useRemovePerson()
  const [editing, setEditing] = useState<Editing | null>(null)
  const [removing, setRemoving] = useState<{ directory: Directory; person: PersonEntry } | null>(null)
  const [blocked, setBlocked] = useState<string[] | null>(null)
  const scope = filters.node
  const shown = directories.filter(
    (directory) => scope === null || isWithinNode(directory.node, scope) || isWithinNode(scope, directory.node),
  )

  const confirmRemove = () => {
    if (removing === null) {
      return
    }
    remove.mutate(
      { node: removing.directory.node, handle: removing.person.handle },
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

  return (
    <div className="flex flex-col gap-6 p-6">
      <PageHeader
        title="People"
        description="Whom items may wait for, declared where a node binds its tracker and shared by everything below it. The header lists only those nodes here."
      />
      {error ? (
        <ErrorState message={`Could not read the people: ${error.message}`} />
      ) : isPending ? (
        <LoadingState rows={3} />
      ) : shown.length === 0 ? (
        <EmptyState icon={Users} title="No tracker here">
          People are declared in a node that binds a tracker (<code>[tracker]</code> in its <code>node.toml</code>).
        </EmptyState>
      ) : (
        shown.map((directory) => (
          <section key={directory.node} className="space-y-2">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <h2 className="text-sm font-semibold">
                <span className="font-mono">{displayNode(directory.node)}</span>
                {directory.account && (
                  <span className="ml-2 font-normal text-muted-foreground">
                    {directory.tracker} · {directory.account}
                  </span>
                )}
              </h2>
              <Button size="sm" variant="outline" onClick={() => setEditing({ directory, person: null })}>
                <Plus className="size-3.5" aria-hidden />
                Add a person
              </Button>
            </div>
            {directory.people.length === 0 ? (
              <p className="rounded-lg border border-dashed border-border p-4 text-sm text-muted-foreground">No one yet.</p>
            ) : (
              <table className="w-full overflow-hidden rounded-lg border border-border text-sm">
                <thead className="bg-muted/50 text-left text-xs text-muted-foreground">
                  <tr>
                    <th className="px-3 py-2 font-medium">Name</th>
                    <th className="px-3 py-2 font-medium">Handle</th>
                    <th className="px-3 py-2 font-medium">monday</th>
                    <th className="px-3 py-2 font-medium">Awaited by</th>
                    <th className="w-20 px-3 py-2" />
                  </tr>
                </thead>
                <tbody>
                  {directory.people.map((person) => (
                    <tr key={person.handle} className="border-t border-border align-top">
                      <td className="px-3 py-2 font-medium">{person.name}</td>
                      <td className="px-3 py-2 font-mono text-xs">{person.handle}</td>
                      <td className="px-3 py-2 font-mono text-xs text-muted-foreground">{person.monday ?? '—'}</td>
                      <td className="px-3 py-2">
                        {person.awaited_by.length === 0 ? (
                          <span className="text-xs text-muted-foreground">—</span>
                        ) : (
                          <ul className="flex flex-wrap gap-1">
                            {person.awaited_by.map((reference) => (
                              <li key={reference}>
                                <ItemReference reference={reference} />
                              </li>
                            ))}
                          </ul>
                        )}
                      </td>
                      <td className="px-3 py-2">
                        <div className="flex justify-end gap-1">
                          <button type="button" className="icon-button size-7" aria-label={`Edit ${person.name}`} onClick={() => setEditing({ directory, person })}>
                            <Pencil className="size-3.5" aria-hidden />
                          </button>
                          <button
                            type="button"
                            className="icon-button size-7 hover:text-red-600"
                            aria-label={`Remove ${person.name}`}
                            onClick={() => setRemoving({ directory, person })}
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
        ))
      )}
      {editing && (
        <PersonDialog
          key={`${editing.directory.node}:${editing.person?.handle ?? 'new'}`}
          node={editing.directory.node}
          account={editing.directory.tracker === 'monday' ? editing.directory.account : null}
          person={editing.person}
          onClose={() => setEditing(null)}
        />
      )}
      <ConfirmDialog
        open={removing !== null}
        onOpenChange={(open) => !open && setRemoving(null)}
        title={`Remove ${removing?.person.name ?? ''}?`}
        description="They leave the directory of this node. Refused while items still wait for them."
        confirmLabel={remove.isPending ? 'Removing…' : 'Remove'}
        pending={remove.isPending}
        onConfirm={confirmRemove}
      />
      <ConfirmDialog
        open={blocked !== null}
        onOpenChange={(open) => !open && setBlocked(null)}
        title="Still awaited"
        description={`These items wait for this person: ${(blocked ?? []).join(', ')}. Take them out of waiting, or wait for someone else, first.`}
        confirmLabel="OK"
        onConfirm={() => setBlocked(null)}
      />
    </div>
  )
}
