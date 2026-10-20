import { GitCommitHorizontal, Upload } from 'lucide-react'
import { useState } from 'react'
import { useRepo } from '@/entities/repo'
import type { UncommittedFile } from '@/shared/api'
import { Button, HoverCard, HoverCardContent, HoverCardTrigger } from '@/shared/ui'
import { usePush } from '../api/use-git-actions'
import { CommitForm } from './commit-form'

export function UncommittedList({ files }: { files: UncommittedFile[] }) {
  return (
    <ul className="max-h-64 space-y-1 overflow-y-auto font-mono text-xs">
      {files.map((file) => (
        <li key={file.path} className="flex gap-2">
          <span className="w-5 shrink-0 text-amber-600 dark:text-amber-400">{file.code}</span>
          <span className="truncate" title={file.path}>
            {file.path}
          </span>
        </li>
      ))}
    </ul>
  )
}

export function UncommittedIndicator() {
  const { data: repo } = useRepo()
  const push = usePush()
  const [composing, setComposing] = useState(false)
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
  return (
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
        {files.length > 0 ? <UncommittedList files={files} /> : null}
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
            </div>
          )}
        </div>
      </HoverCardContent>
    </HoverCard>
  )
}
