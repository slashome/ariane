import { useState } from 'react'
import { useAddPerson, useChangePerson } from '../api/use-people'
import { isHandle, suggestHandle } from '../model/handle'
import { findMondayUsers, type MondayUser, type PersonEntry } from '@/shared/api'
import { Button, Dialog, Icon, Input } from '@/shared/ui'
import './people.css'

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
      className="people-dialog"
    >
      <form
        className="people-form"
        onSubmit={(event) => {
          event.preventDefault()
          save()
        }}
      >
        <label className="people-form__field">
          <span className="people-form__label">Name</span>
          <Input value={name} onChange={(event) => setName(event.target.value)} autoFocus />
        </label>
        <label className="people-form__field">
          <span className="people-form__label">Handle</span>
          <Input
            value={shownHandle}
            className="people-form__mono"
            aria-invalid={shownHandle !== '' && !isHandle(shownHandle)}
            onChange={(event) => {
              setHandleTouched(true)
              setHandle(event.target.value)
            }}
          />
          <span className="people-form__hint">
            What items write in <code>waiting_for</code>.
            {person !== null && shownHandle !== person.handle && ` Renaming rewrites the ${person.awaited_by.length} items that wait for them.`}
          </span>
        </label>
        <div className="people-form__field">
          <label htmlFor="person-monday" className="people-form__label">
            monday id
          </label>
          <Input id="person-monday" inputMode="numeric" className="people-form__mono" value={monday} onChange={(event) => setMonday(event.target.value)} />
          {account !== null && (
            <div className="person-search">
              <div className="person-search__row">
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
                  <Icon name="search" className="person-search__icon" />
                </Button>
              </div>
              {searchError && <p className="person-search__error">{searchError}</p>}
              {found !== null && (
                <ul className="person-search__results">
                  {found.length === 0 && <li className="person-search__none">No one by that name</li>}
                  {found.map((user) => (
                    <li key={user.id}>
                      <button
                        type="button"
                        className="person-search__result"
                        onClick={() => {
                          setMonday(String(user.id))
                          if (name.trim() === '') {
                            setName(user.name)
                          }
                        }}
                      >
                        <span>{user.name}</span>
                        <span className="person-search__result-id">{user.id}</span>
                      </button>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          )}
        </div>
        <div className="people-form__actions">
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
