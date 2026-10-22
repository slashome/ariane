import { useDiff } from '@/entities/repo'
import { cn } from '@/shared/lib'
import { Dialog } from '@/shared/ui'
import { diffLines, type DiffLineKind } from '../model/diff-lines'

const toneOf: Record<DiffLineKind, string> = {
  added: 'bg-green-500/15 text-green-800 dark:text-green-300',
  removed: 'bg-red-500/15 text-red-800 dark:text-red-300',
  hunk: 'text-sky-700 dark:text-sky-300',
  meta: 'text-muted-foreground',
  context: '',
}

export function DiffDialog({ path, onClose }: { path: string | null; onClose: () => void }) {
  const { data, isPending, error } = useDiff(path)
  const lines = data ? diffLines(data.diff) : []
  return (
    <Dialog open={path !== null} onOpenChange={(open) => !open && onClose()} title={path ?? ''} description="Changes against HEAD">
      <div className="min-h-0 flex-1 overflow-auto rounded-md border border-border bg-muted/30">
        {error ? (
          <p role="alert" className="p-3 text-sm">
            Could not load the diff: {error.message}
          </p>
        ) : isPending ? (
          <p className="p-3 text-sm text-muted-foreground">Loading…</p>
        ) : lines.length === 0 ? (
          <p className="p-3 text-sm text-muted-foreground">No difference with HEAD.</p>
        ) : (
          <pre className="py-2 font-mono text-xs leading-5">
            {lines.map((line, index) => (
              <div key={index} className={cn('px-3 whitespace-pre-wrap break-all', toneOf[line.kind])}>
                {line.text === '' ? ' ' : line.text}
              </div>
            ))}
          </pre>
        )}
      </div>
    </Dialog>
  )
}
