import { useState } from 'react'
import type { UncommittedFile } from '@/shared/api'
import { Button, Input } from '@/shared/ui'
import { useCommit } from '../api/use-git-actions'
import { defaultCommitMessage } from '../model/commit-message'
import './uncommitted-indicator.css'

export function CommitForm({ files, onDone }: { files: UncommittedFile[]; onDone: () => void }) {
  const [message, setMessage] = useState(() => defaultCommitMessage(files))
  const commit = useCommit()
  return (
    <form
      className="commit-form"
      onSubmit={(event) => {
        event.preventDefault()
        commit.mutate(message.trim(), { onSuccess: onDone })
      }}
    >
      <Input aria-label="Commit message" value={message} onChange={(event) => setMessage(event.target.value)} autoFocus />
      <div className="commit-form__actions">
        <Button size="sm" variant="ghost" onClick={onDone}>
          Cancel
        </Button>
        <Button size="sm" type="submit" disabled={message.trim() === '' || commit.isPending}>
          {commit.isPending ? 'Committing…' : 'Commit'}
        </Button>
      </div>
    </form>
  )
}
