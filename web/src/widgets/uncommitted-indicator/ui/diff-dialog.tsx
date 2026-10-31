import { useDiff } from '@/entities/repo'
import { cn } from '@/shared/lib'
import { Dialog } from '@/shared/ui'
import { diffLines } from '../model/diff-lines'
import './uncommitted-indicator.css'

export function DiffDialog({ path, onClose }: { path: string | null; onClose: () => void }) {
  const { data, isPending, error } = useDiff(path)
  const lines = data ? diffLines(data.diff) : []
  return (
    <Dialog open={path !== null} onOpenChange={(open) => !open && onClose()} title={path ?? ''} description="Changes against HEAD">
      <div className="diff-dialog__body">
        {error ? (
          <p role="alert" className="diff-dialog__message">
            Could not load the diff: {error.message}
          </p>
        ) : isPending ? (
          <p className="diff-dialog__message diff-dialog__message--muted">Loading…</p>
        ) : lines.length === 0 ? (
          <p className="diff-dialog__message diff-dialog__message--muted">No difference with HEAD.</p>
        ) : (
          <pre className="diff-dialog__diff">
            {lines.map((line, index) => (
              <div key={index} className={cn('diff-dialog__line', `diff-dialog__line--${line.kind}`)}>
                {line.text === '' ? ' ' : line.text}
              </div>
            ))}
          </pre>
        )}
      </div>
    </Dialog>
  )
}
