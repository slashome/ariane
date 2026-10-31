import { Pencil, Plus, Trash2 } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useSearchParams } from 'react-router'
import { usePeople, useRemovePerson } from '../api/use-people'
import { usePersonas } from '../api/use-personas'
import { useItems } from '@/entities/item'
import { QueryField, useFilteredItems, useItemFilters } from '@/features/filter-items'
import { ApiError, type Directory, type PersonEntry } from '@/shared/api'
import { cn, displayNode, isWithinNode } from '@/shared/lib'
import { Button, ConfirmDialog, DataTable, EmptyState, ErrorState, Highlight, LoadingState, PageHeader, type Column } from '@/shared/ui'
import { References } from './item-reference'
import { PersonasList } from './personas-list'
import { PersonDialog } from './person-dialog'

type Editing = { directory: Directory; person: PersonEntry | null }

// The two directories the page edits, the current one kept in the URL.
const tabs = [
  { value: 'people', label: 'People', description: 'Whom items may wait for, declared where a node binds its tracker and shared by everything below it.' },
  { value: 'personas', label: 'Personas', description: 'Whom stories are written for, declared on a node and shared by everything below it.' },
] as const
const tabParam = 'tab'

export function PeoplePage() {
  const [params, setParams] = useSearchParams()
  const tab = tabs.find((entry) => entry.value === params.get(tabParam)) ?? tabs[0]
  const { data: directories = [], isPending, error } = usePeople()
  const personas = usePersonas()
  const { filters } = useItemFilters()
  const remove = useRemovePerson()
  const [editing, setEditing] = useState<Editing | null>(null)
  const [removing, setRemoving] = useState<{ directory: Directory; person: PersonEntry } | null>(null)
  const [blocked, setBlocked] = useState<string[] | null>(null)
  const scope = filters.node
  const { data: items = [] } = useItems()
  const { items: matching } = useFilteredItems(items, { anyNode: true })
  const searching = filters.query.trim() !== ''
  // With a search, each person keeps only the awaiting items it finds, and people left with none are hidden.
  const shown = useMemo(() => {
    const found = new Set(matching.map((item) => item.reference))
    return directories
      .filter((directory) => scope === null || isWithinNode(directory.node, scope) || isWithinNode(scope, directory.node))
      .map((directory) => ({
        ...directory,
        people: searching
          ? directory.people
              .map((person) => ({ ...person, awaited_by: person.awaited_by.filter((reference) => found.has(reference)) }))
              .filter((person) => person.awaited_by.length > 0)
          : directory.people,
      }))
  }, [directories, matching, scope, searching])
  const awaited = (list: Directory[]) => list.reduce((sum, directory) => sum + directory.people.reduce((count, person) => count + person.awaited_by.length, 0), 0)
  const total = awaited(directories.filter((directory) => shown.some((entry) => entry.node === directory.node)))
  // The same scope and search for the personas: with a search, each keeps only the stories it finds.
  const personaDirectories = useMemo(() => {
    const found = new Set(matching.map((item) => item.reference))
    return (personas.data ?? [])
      .filter((directory) => scope === null || isWithinNode(directory.node, scope) || isWithinNode(scope, directory.node))
      .map((directory) => ({
        ...directory,
        personas: searching
          ? directory.personas
              .map((persona) => ({ ...persona, used_by: persona.used_by.filter((reference) => found.has(reference)) }))
              .filter((persona) => persona.used_by.length > 0)
          : directory.personas,
      }))
  }, [personas.data, matching, scope, searching])
  const written = (list: { personas: { used_by: string[] }[] }[]) => list.reduce((sum, directory) => sum + directory.personas.reduce((count, persona) => count + persona.used_by.length, 0), 0)
  const writtenTotal = written((personas.data ?? []).filter((directory) => personaDirectories.some((entry) => entry.node === directory.node)))

  const selectTab = (value: string) =>
    setParams(
      (current) => {
        const next = new URLSearchParams(current)
        if (value === tabs[0].value) {
          next.delete(tabParam)
        } else {
          next.set(tabParam, value)
        }
        return next
      },
      { replace: true },
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

  const columnsFor = (directory: Directory): Column<PersonEntry>[] => [
    {
      id: 'name',
      header: 'Name',
      sortValue: (person) => person.name,
      className: 'font-medium',
      cell: (person) => <Highlight text={person.name} />,
    },
    {
      id: 'handle',
      header: 'Handle',
      sortValue: (person) => person.handle,
      className: 'font-mono text-xs',
      cell: (person) => <Highlight text={person.handle} />,
    },
    {
      id: 'monday',
      header: 'monday',
      sortValue: (person) => person.monday,
      className: 'font-mono text-xs text-muted-foreground',
      cell: (person) => person.monday ?? '—',
    },
    {
      id: 'awaited',
      header: 'Awaited by',
      sortValue: (person) => person.awaited_by.length || null,
      cell: (person) => <References references={person.awaited_by} />,
    },
    {
      id: 'actions',
      header: '',
      headerClassName: 'w-20',
      cell: (person) => (
        <div className="flex justify-end gap-1">
          <button type="button" className="lu-icon-button size-7" aria-label={`Edit ${person.name}`} onClick={() => setEditing({ directory, person })}>
            <Pencil className="size-3.5" aria-hidden />
          </button>
          <button
            type="button"
            className="lu-icon-button size-7 hover:text-red-600"
            aria-label={`Remove ${person.name}`}
            onClick={() => setRemoving({ directory, person })}
          >
            <Trash2 className="size-3.5" aria-hidden />
          </button>
        </div>
      ),
    },
  ]

  return (
    <div className="flex flex-col gap-6 p-6">
      <PageHeader
        title="People"
        description={tab.description}
        actions={
          tab.value === 'personas' ? (
            <QueryField count={written(personaDirectories)} total={writtenTotal} />
          ) : (
            <QueryField count={awaited(shown)} total={total} />
          )
        }
      />
      <div role="tablist" aria-label="Directories" className="flex gap-1 border-b border-border">
        {tabs.map((entry) => (
          <button
            key={entry.value}
            type="button"
            role="tab"
            aria-selected={tab.value === entry.value}
            onClick={() => selectTab(entry.value)}
            className={cn(
              '-mb-px border-b-2 border-transparent px-3 py-1.5 text-sm text-muted-foreground hover:text-foreground',
              tab.value === entry.value && 'border-primary font-medium text-foreground',
            )}
          >
            {entry.label}
          </button>
        ))}
      </div>
      {tab.value === 'personas' ? (
        personas.error ? (
          <ErrorState message={`Could not read the personas: ${personas.error.message}`} />
        ) : personas.isPending ? (
          <LoadingState rows={3} />
        ) : (
          <PersonasList directories={personaDirectories} searching={searching} />
        )
      ) : error ? (
        <ErrorState message={`Could not read the people: ${error.message}`} />
      ) : isPending ? (
        <LoadingState rows={3} />
      ) : shown.length === 0 ? (
        <EmptyState icon="users" title="No tracker here">
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
              <p className="rounded-lg border border-dashed border-border p-4 text-sm text-muted-foreground">
                {searching ? 'No one here waited on by an item the search finds.' : 'No one yet.'}
              </p>
            ) : (
              <DataTable
                rows={directory.people}
                columns={columnsFor(directory)}
                rowKey={(person) => person.handle}
                storageKey="people"
                label={`People of ${displayNode(directory.node)}`}
              />
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
