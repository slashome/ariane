import { useState } from 'react'
import { useRepo } from '@/entities/repo'
import { Button, ConfirmDialog, EmptyState, ErrorState, Icon, LoadingState } from '@/shared/ui'
import { useDiscard, usePull, usePush } from '../api/use-git-actions'
import { CommitForm } from './commit-form'
import { DiffDialog } from './diff-dialog'
import { UncommittedList } from './uncommitted-indicator'
import './uncommitted-indicator.css'

// The whole state of the content repository, for the Repository page: what the header's indicator
// shows on hover, with room to read it.
export function RepoPanel() {
  const { data: repo, isPending, error } = useRepo()
  const push = usePush()
  const pull = usePull()
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
    <div className="repo-panel">
      <dl className="repo-panel__stats">
        <div className="repo-panel__stat">
          <dt className="repo-panel__stat-label">
            <Icon name="branch" className="uncommitted-indicator__icon" /> Branch
          </dt>
          <dd className="repo-panel__stat-value repo-panel__stat-value--mono">{repo.branch ?? 'detached'}</dd>
        </div>
        <div className="repo-panel__stat">
          <dt className="repo-panel__stat-label">
            <Icon name="commit" className="uncommitted-indicator__icon" /> Uncommitted files
          </dt>
          <dd className="repo-panel__stat-value">{files.length}</dd>
        </div>
        <div className="repo-panel__stat">
          <dt className="repo-panel__stat-label">
            <Icon name="upload" className="uncommitted-indicator__icon" /> Commits to push
          </dt>
          <dd className="repo-panel__stat-value">{repo.ahead}</dd>
        </div>
      </dl>
      <p className="repo-panel__root">
        <Icon name="repository" className="uncommitted-indicator__icon" />
        {repo.root}
      </p>
      <section className="repo-panel__tree">
        <div className="repo-panel__toolbar">
          <h2 className="repo-panel__title">Working tree</h2>
          <div className="repo-panel__actions">
            {files.length > 0 && !composing && (
              <Button size="sm" onClick={() => setComposing(true)}>
                <Icon name="commit" className="uncommitted-indicator__icon" />
                Commit all
              </Button>
            )}
            <Button size="sm" variant="outline" disabled={repo.ahead === 0 || push.isPending} onClick={() => push.mutate()}>
              <Icon name="upload" className="uncommitted-indicator__icon" />
              {push.isPending ? 'Pushing…' : 'Push'}
            </Button>
            <Button size="sm" variant="outline" disabled={pull.isPending} onClick={() => pull.mutate()}>
              <Icon name="download" className="uncommitted-indicator__icon" />
              {pull.isPending ? 'Pulling…' : 'Pull'}
            </Button>
            {files.some((file) => file.path.startsWith('tree/') || file.path === 'settings.toml') && (
              <Button size="sm" variant="ghost" className="uncommitted-indicator__discard" onClick={() => setDiscarding({ path: null })}>
                <Icon name="undo" className="uncommitted-indicator__icon" />
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
