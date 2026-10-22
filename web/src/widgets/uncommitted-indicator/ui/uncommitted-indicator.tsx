import { GitCommitHorizontal, Undo2, Upload } from 'lucide-react'
import { useState } from 'react'
import { useRepo } from '@/entities/repo'
import type { UncommittedFile } from '@/shared/api'
import { Button, ConfirmDialog, HoverCard, HoverCardContent, HoverCardTrigger } from '@/shared/ui'
import { useDiscard, usePush } from '../api/use-git-actions'
import { CommitForm } from './commit-form'
import { DiffDialog } from './diff-dialog'

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
    <ul className="max-h-64 space-y-0.5 overflow-y-auto font-mono text-xs">
      {files.map((file) => (
        <li key={file.path} className="group flex items-center gap-2">
          <span className="w-5 shrink-0 text-amber-600 dark:text-amber-400">{file.code}</span>
          <button
            type="button"
            className="min-w-0 flex-1 truncate rounded px-1 py-0.5 text-left hover:bg-muted"
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
              className="icon-button mr-1 size-4.5 shrink-0 hover:text-red-600"
              onClick={() => onDiscard(file.path)}
            >
              <Undo2 className="size-3.5" aria-hidden />
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
      <HoverCard openDelay={100} open={composing ? true : undefined}>
        <HoverCardTrigger asChild>
          <button
            type="button"
            aria-label={label}
            className="inline-flex items-center gap-1.5 rounded-full bg-amber-500/15 px-2.5 py-1 text-xs font-medium text-amber-700 dark:text-amber-300"
          >
            {files.length > 0 ? (
              <>
                <GitCommitHorizontal className="size-3.5" aria-hidden />
                {files.length}
              </>
            ) : null}
            {ahead > 0 ? (
              <>
                <Upload className="size-3.5" aria-hidden />
                {ahead}
              </>
            ) : null}
          </button>
        </HoverCardTrigger>
        <HoverCardContent onEscapeKeyDown={() => setComposing(false)}>
          <p className="mb-2 font-medium">{label}</p>
          {files.length > 0 ? (
            <UncommittedList files={files} onView={setViewing} onDiscard={(path) => setDiscarding({ path })} />
          ) : null}
          <div className="mt-3 border-t border-border pt-3">
            {composing ? (
              <CommitForm files={files} onDone={() => setComposing(false)} />
            ) : (
              <div className="flex gap-2">
                {files.length > 0 ? (
                  <Button size="sm" onClick={() => setComposing(true)}>
                    <GitCommitHorizontal className="size-3.5" aria-hidden />
                    Commit all
                  </Button>
                ) : null}
                <Button
                  size="sm"
                  variant="outline"
                  disabled={ahead === 0 || push.isPending}
                  onClick={() => push.mutate()}
                >
                  <Upload className="size-3.5" aria-hidden />
                  {push.isPending ? 'Pushing…' : 'Push'}
                </Button>
                {files.some(discardable) ? (
                  <Button
                    size="sm"
                    variant="ghost"
                    className="ml-auto text-red-600 hover:text-red-700 dark:text-red-400"
                    onClick={() => setDiscarding({ path: null })}
                  >
                    <Undo2 className="size-3.5" aria-hidden />
                    Discard all
                  </Button>
                ) : null}
              </div>
            )}
          </div>
        </HoverCardContent>
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
