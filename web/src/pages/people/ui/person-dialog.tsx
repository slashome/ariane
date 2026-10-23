import { Search } from 'lucide-react'
import { useState } from 'react'
import { useAddPerson, useChangePerson } from '../api/use-people'
import { isHandle, suggestHandle } from '../model/handle'
import { findMondayUsers, type MondayUser, type PersonEntry } from '@/shared/api'
import { Button, Dialog, Input } from '@/shared/ui'

// Adds a person to a node's directory, or edits one: name, handle (renamed in every item that waits
// for them) and monday id, typed or found by name in the account.
export function PersonDialog({
  node,
  account,
  person,
  onClose,
}: {
  node: string
  account: string | null
  // The person to edit; `null` to add one.
  person: PersonEntry | null
  onClose: () => void
}) {
  const add = useAddPerson()
  const change = useChangePerson()
  const [name, setName] = useState(person?.name ?? '')
  const [handle, setHandle] = useState(person?.handle ?? '')
  const [handleTouched, setHandleTouched] = useState(person !== null)
  const [monday, setMonday] = useState(person?.monday?.toString() ?? '')
  const [query, setQuery] = useState('')
  const [found, setFound] = useState<MondayUser[] | null>(null)
  const [searchError, setSearchError] = useState<string | null>(null)

  const shownHandle = handleTouched ? handle : (suggestHandle(name) ?? '')
  const mondayId = monday.trim() === '' ? null : Number(monday)
  const valid = name.trim() !== '' && isHandle(shownHandle) && (mondayId === null || Number.isInteger(mondayId))

  const search = () => {
    if (account === null || query.trim() === '') {
      return
    }
    setSearchError(null)
    findMondayUsers(account, query.trim())
      .then(setFound)
      .catch((error: unknown) => setSearchError(error instanceof Error ? error.message : String(error)))
  }

  const save = () => {
    if (!valid) {
      return
    }
    const done = { onSuccess: onClose }
    if (person === null) {
      add.mutate({ node, handle: shownHandle, name: name.trim(), monday: mondayId }, done)
    } else {
      change.mutate(
        {
          node,
          handle: person.handle,
          name: name.trim(),
          monday: mondayId,
          rename_to: shownHandle !== person.handle ? shownHandle : undefined,
        },
        done,
      )
    }
  }

  return (
    <Dialog
      open
      onOpenChange={(open) => !open && onClose()}
      title={person === null ? 'Add a person' : `Edit ${person.name}`}
      description={`Declared in ${node === '' ? '(home)' : node}, for every item below it.`}
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
          <span className="font-medium">Handle</span>
          <Input
            value={shownHandle}
            className="font-mono"
            aria-invalid={shownHandle !== '' && !isHandle(shownHandle)}
            onChange={(event) => {
              setHandleTouched(true)
              setHandle(event.target.value)
            }}
          />
          <span className="block text-xs text-muted-foreground">
            What items write in <code>waiting_for</code>.
            {person !== null && shownHandle !== person.handle && ` Renaming rewrites the ${person.awaited_by.length} items that wait for them.`}
          </span>
        </label>
        <div className="space-y-1 text-sm">
          <label htmlFor="person-monday" className="font-medium">
            monday id
          </label>
          <Input id="person-monday" inputMode="numeric" className="font-mono" value={monday} onChange={(event) => setMonday(event.target.value)} />
          {account !== null && (
            <div className="space-y-1.5 pt-1">
              <div className="flex gap-2">
                <Input
                  aria-label="Search monday users by name"
                  placeholder={`Search ${account} by name…`}
                  value={query}
                  onChange={(event) => setQuery(event.target.value)}
                  onKeyDown={(event) => {
                    if (event.key === 'Enter') {
                      event.preventDefault()
                      search()
                    }
                  }}
                />
                <Button variant="outline" onClick={search} aria-label="Search monday">
                  <Search className="size-4" aria-hidden />
                </Button>
              </div>
              {searchError && <p className="text-xs text-red-700 dark:text-red-300">{searchError}</p>}
              {found !== null && (
                <ul className="max-h-40 overflow-y-auto rounded-md border border-border">
                  {found.length === 0 && <li className="px-2 py-1.5 text-xs text-muted-foreground">No one by that name</li>}
                  {found.map((user) => (
                    <li key={user.id}>
                      <button
                        type="button"
                        className="flex w-full items-baseline justify-between gap-2 px-2 py-1.5 text-left text-sm hover:bg-muted"
                        onClick={() => {
                          setMonday(String(user.id))
                          if (name.trim() === '') {
                            setName(user.name)
                          }
                        }}
                      >
                        <span>{user.name}</span>
                        <span className="font-mono text-[10px] text-muted-foreground">{user.id}</span>
                      </button>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          )}
        </div>
        <div className="flex justify-end gap-2">
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button type="submit" disabled={!valid || add.isPending || change.isPending}>
            {person === null ? 'Add' : 'Save'}
          </Button>
        </div>
      </form>
    </Dialog>
  )
}
