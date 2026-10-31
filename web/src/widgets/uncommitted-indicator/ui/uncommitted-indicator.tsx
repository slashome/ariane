import { useState } from 'react'
import { useRepo } from '@/entities/repo'
import type { UncommittedFile } from '@/shared/api'
import { Button, ConfirmDialog, HoverCard, Icon } from '@/shared/ui'
import { useDiscard, usePush } from '../api/use-git-actions'
import { CommitForm } from './commit-form'
import { DiffDialog } from './diff-dialog'
import './uncommitted-indicator.css'

const discardable = (file: UncommittedFile) => file.path.startsWith('tree/') || file.path === 'settings.toml'

export function UncommittedList({
  files,
  onView,
  onDiscard,
}: {
  files: UncommittedFile[]
  onView: (path: string) => void
  onDiscard: (path: string) => void
}) {
  return (
    <ul className="uncommitted-list">
      {files.map((file) => (
        <li key={file.path} className="uncommitted-list__file">
          <span className="uncommitted-list__code">{file.code}</span>
          <button
            type="button"
            className="uncommitted-list__path lu-truncate"
            title={`Show the diff of ${file.path}`}
            onClick={() => onView(file.path)}
          >
            {file.path}
          </button>
          {discardable(file) ? (
            <button
              type="button"
              aria-label={`Discard ${file.path}`}
              title="Discard this change"
              className="lu-icon-button uncommitted-list__discard"
              onClick={() => onDiscard(file.path)}
            >
              <Icon name="undo" className="uncommitted-indicator__icon" />
            </button>
          ) : null}
        </li>
      ))}
    </ul>
  )
}

// The hover card's content unmounts as soon as the pointer leaves it, so the dialogs it opens keep
// their state here and are rendered next to the hover card, not inside it.
export function UncommittedIndicator() {
  const { data: repo } = useRepo()
  const push = usePush()
  const discard = useDiscard()
  const [composing, setComposing] = useState(false)
  const [viewing, setViewing] = useState<string | null>(null)
  const [discarding, setDiscarding] = useState<{ path: string | null } | null>(null)
  const files = repo?.uncommitted ?? []
  const ahead = repo?.ahead ?? 0
  if (files.length === 0 && ahead === 0) {
    return null
  }
  const pending = [
    files.length > 0 ? `${files.length} uncommitted ${files.length === 1 ? 'file' : 'files'}` : null,
    ahead > 0 ? `${ahead} ${ahead === 1 ? 'commit' : 'commits'} to push` : null,
  ].filter(Boolean)
  const label = pending.join(', ')
  const confirmDiscard = () => {
    if (discarding === null) {
      return
    }
    discard.mutate(discarding.path === null ? undefined : [discarding.path], {
      onSettled: () => setDiscarding(null),
    })
  }
  return (
    <>
      <HoverCard
        openDelay={100}
        pinned={composing}
        trigger={
          <button
            type="button"
            aria-label={label}
            className="uncommitted-indicator__trigger"
          >
            {files.length > 0 ? (
              <>
                <Icon name="commit" className="uncommitted-indicator__icon" />
                {files.length}
              </>
            ) : null}
            {ahead > 0 ? (
              <>
                <Icon name="upload" className="uncommitted-indicator__icon" />
                {ahead}
              </>
            ) : null}
          </button>
        }
      >
        <div onKeyDown={(event) => event.key === 'Escape' && setComposing(false)}>
          <p className="uncommitted-indicator__label">{label}</p>
          {files.length > 0 ? (
            <UncommittedList files={files} onView={setViewing} onDiscard={(path) => setDiscarding({ path })} />
          ) : null}
          <div className="uncommitted-indicator__actions">
            {composing ? (
              <CommitForm files={files} onDone={() => setComposing(false)} />
            ) : (
              <div className="uncommitted-indicator__buttons">
                {files.length > 0 ? (
                  <Button size="sm" onClick={() => setComposing(true)}>
                    <Icon name="commit" className="uncommitted-indicator__icon" />
                    Commit all
                  </Button>
                ) : null}
                <Button
                  size="sm"
                  variant="outline"
                  disabled={ahead === 0 || push.isPending}
                  onClick={() => push.mutate()}
                >
                  <Icon name="upload" className="uncommitted-indicator__icon" />
                  {push.isPending ? 'Pushing…' : 'Push'}
                </Button>
                {files.some(discardable) ? (
                  <Button
                    size="sm"
                    variant="ghost"
                    className="uncommitted-indicator__discard uncommitted-indicator__discard--end"
                    onClick={() => setDiscarding({ path: null })}
                  >
                    <Icon name="undo" className="uncommitted-indicator__icon" />
                    Discard all
                  </Button>
                ) : null}
              </div>
            )}
          </div>
        </div>
      </HoverCard>
      <ConfirmDialog
        open={discarding !== null}
        onOpenChange={(open) => !open && setDiscarding(null)}
        title={discarding?.path ? 'Discard this change?' : 'Discard all changes?'}
        description={
          discarding?.path
            ? `${discarding.path} goes back to its committed version, or is deleted if it was never committed. This cannot be undone.`
            : 'Every uncommitted change under tree/ is lost: modified files go back to their committed version, new files are deleted. This cannot be undone.'
        }
        confirmLabel={discard.isPending ? 'Discarding…' : 'Discard'}
        pending={discard.isPending}
        onConfirm={confirmDiscard}
      />
      <DiffDialog path={viewing} onClose={() => setViewing(null)} />
    </>
  )
}
