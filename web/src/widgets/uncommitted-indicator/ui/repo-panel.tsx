import { FolderGit2, GitBranch, GitCommitHorizontal, Undo2, Upload } from 'lucide-react'
import { useState } from 'react'
import { useRepo } from '@/entities/repo'
import { Button, ConfirmDialog, EmptyState, ErrorState, LoadingState } from '@/shared/ui'
import { useDiscard, usePush } from '../api/use-git-actions'
import { CommitForm } from './commit-form'
import { DiffDialog } from './diff-dialog'
import { UncommittedList } from './uncommitted-indicator'

// The whole state of the content repository, for the Repository page: what the header's indicator
// shows on hover, with room to read it.
export function RepoPanel() {
  const { data: repo, isPending, error } = useRepo()
  const push = usePush()
  const discard = useDiscard()
  const [viewing, setViewing] = useState<string | null>(null)
  const [discarding, setDiscarding] = useState<{ path: string | null } | null>(null)
  const [composing, setComposing] = useState(false)
  if (error) {
    return <ErrorState message={`Could not read the repository: ${error.message}`} />
  }
  if (isPending) {
    return <LoadingState rows={3} />
  }
  const files = repo.uncommitted
  return (
    <div className="flex max-w-4xl flex-col gap-5">
      <dl className="grid gap-3 sm:grid-cols-3">
        <div className="rounded-lg border border-border p-3">
          <dt className="flex items-center gap-1 text-xs text-muted-foreground">
            <GitBranch className="size-3.5" aria-hidden /> Branch
          </dt>
          <dd className="font-mono text-sm">{repo.branch ?? 'detached'}</dd>
        </div>
        <div className="rounded-lg border border-border p-3">
          <dt className="flex items-center gap-1 text-xs text-muted-foreground">
            <GitCommitHorizontal className="size-3.5" aria-hidden /> Uncommitted files
          </dt>
          <dd className="text-2xl font-semibold tabular-nums">{files.length}</dd>
        </div>
        <div className="rounded-lg border border-border p-3">
          <dt className="flex items-center gap-1 text-xs text-muted-foreground">
            <Upload className="size-3.5" aria-hidden /> Commits to push
          </dt>
          <dd className="text-2xl font-semibold tabular-nums">{repo.ahead}</dd>
        </div>
      </dl>
      <p className="flex items-center gap-1.5 font-mono text-xs text-muted-foreground">
        <FolderGit2 className="size-3.5" aria-hidden />
        {repo.root}
      </p>
      <section className="space-y-3 rounded-lg border border-border p-4">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h2 className="text-sm font-semibold">Working tree</h2>
          <div className="flex gap-2">
            {files.length > 0 && !composing && (
              <Button size="sm" onClick={() => setComposing(true)}>
                <GitCommitHorizontal className="size-3.5" aria-hidden />
                Commit all
              </Button>
            )}
            <Button size="sm" variant="outline" disabled={repo.ahead === 0 || push.isPending} onClick={() => push.mutate()}>
              <Upload className="size-3.5" aria-hidden />
              {push.isPending ? 'Pushing…' : 'Push'}
            </Button>
            {files.some((file) => file.path.startsWith('tree/')) && (
              <Button size="sm" variant="ghost" className="text-red-600 dark:text-red-400" onClick={() => setDiscarding({ path: null })}>
                <Undo2 className="size-3.5" aria-hidden />
                Discard all
              </Button>
            )}
          </div>
        </div>
        {composing && <CommitForm files={files} onDone={() => setComposing(false)} />}
        {files.length === 0 ? (
          <EmptyState title="Nothing to commit">The working tree matches the last commit.</EmptyState>
        ) : (
          <UncommittedList files={files} onView={setViewing} onDiscard={(path) => setDiscarding({ path })} />
        )}
      </section>
      <ConfirmDialog
        open={discarding !== null}
        onOpenChange={(open) => !open && setDiscarding(null)}
        title={discarding?.path ? 'Discard this change?' : 'Discard all changes?'}
        description={
          discarding?.path
            ? `${discarding.path} goes back to its committed version, or is deleted if it was never committed. This cannot be undone.`
            : 'Every uncommitted change under tree/ is lost. This cannot be undone.'
        }
        confirmLabel={discard.isPending ? 'Discarding…' : 'Discard'}
        pending={discard.isPending}
        onConfirm={() =>
          discard.mutate(discarding?.path ? [discarding.path] : undefined, { onSettled: () => setDiscarding(null) })
        }
      />
      <DiffDialog path={viewing} onClose={() => setViewing(null)} />
    </div>
  )
}
