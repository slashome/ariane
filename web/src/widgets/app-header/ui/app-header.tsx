import { FolderGit2 } from 'lucide-react'
import type { ReactNode } from 'react'
import { useRepo } from '@/entities/repo'
import { ItemSearchInput } from '@/features/filter-items'
import { NodeSwitcher } from './node-switcher'

export function AppHeader({ actions }: { actions?: ReactNode }) {
  const { data: repo } = useRepo()
  return (
    <header className="flex h-14 shrink-0 items-center gap-3 border-b border-border px-4">
      <div className="flex min-w-0 items-center gap-1.5 text-sm text-muted-foreground" title={repo?.root}>
        <FolderGit2 className="size-4 shrink-0" aria-hidden />
        <span className="truncate font-mono text-xs">{repo?.root ?? '…'}</span>
        {repo?.branch && <span className="rounded bg-muted px-1.5 py-0.5 font-mono text-xs">{repo.branch}</span>}
      </div>
      <NodeSwitcher />
      <div className="ml-auto flex items-center gap-3">
        <ItemSearchInput />
        {actions}
      </div>
    </header>
  )
}
