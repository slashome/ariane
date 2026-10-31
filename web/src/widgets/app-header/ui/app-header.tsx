import type { ReactNode } from 'react'
import { useRepo } from '@/entities/repo'
import { QueryField } from '@/features/filter-items'
import { Icon, Tooltip } from '@/shared/ui'
import { NodeSwitcher } from './node-switcher'
import './app-header.css'

export function AppHeader({ actions }: { actions?: ReactNode }) {
  const { data: repo } = useRepo()
  return (
    <header className="app-header">
      <Tooltip content={repo?.root ?? 'Content repository'}>
        <button
          type="button"
          aria-label={`Content repository: ${repo?.root ?? 'loading'}`}
          className="lu-icon-button app-header__repo"
        >
          <Icon name="repository" className="app-header__icon" />
        </button>
      </Tooltip>
      <NodeSwitcher />
      <div className="app-header__actions">
        <QueryField />
        {actions}
      </div>
    </header>
  )
}
