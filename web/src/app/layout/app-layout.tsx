import { Outlet } from 'react-router'
import { QueryHighlight } from '@/features/filter-items'
import { AppHeader } from '@/widgets/app-header'
import { CommandPalette } from '@/widgets/command-palette'
import { Sidebar } from '@/widgets/sidebar'
import { StatusBar } from '@/widgets/status-bar'
import { UncommittedIndicator } from '@/widgets/uncommitted-indicator'
import { ItemPreview } from './item-preview'
import './layout.css'

export function AppLayout() {
  return (
    <div className="app-layout">
      <div className="app-layout__body">
        <Sidebar />
        <div className="app-layout__column">
          <AppHeader
            actions={
              <>
                <CommandPalette />
                <UncommittedIndicator />
              </>
            }
          />
          <main className="app-layout__main">
            <QueryHighlight>
              <Outlet />
              <ItemPreview />
            </QueryHighlight>
          </main>
        </div>
      </div>
      <StatusBar />
    </div>
  )
}
