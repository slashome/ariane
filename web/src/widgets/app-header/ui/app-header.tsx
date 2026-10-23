import { FolderGit2 } from 'lucide-react'
import type { ReactNode } from 'react'
import { useRepo } from '@/entities/repo'
import { QueryField } from '@/features/filter-items'
import { Tooltip } from '@/shared/ui'
import { NodeSwitcher } from './node-switcher'

export function AppHeader({ actions }: { actions?: ReactNode }) {
  const { data: repo } = useRepo()
  return (
    <header className="flex h-14 shrink-0 items-center gap-3 border-b border-border px-4">
      <Tooltip content={repo?.root ?? 'Content repository'}>
        <button
          type="button"
          aria-label={`Content repository: ${repo?.root ?? 'loading'}`}
          className="icon-button size-7"
        >
          <FolderGit2 className="size-4" aria-hidden />
        </button>
      </Tooltip>
      <NodeSwitcher />
      <div className="ml-auto flex items-center gap-3">
        <QueryField />
        {actions}
      </div>
    </header>
  )
}
