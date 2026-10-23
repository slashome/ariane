import { Outlet } from 'react-router'
import { QueryHighlight } from '@/features/filter-items'
import { AppHeader } from '@/widgets/app-header'
import { Sidebar } from '@/widgets/sidebar'
import { StatusBar } from '@/widgets/status-bar'
import { UncommittedIndicator } from '@/widgets/uncommitted-indicator'

export function AppLayout() {
  return (
    <div className="flex h-screen min-w-[64rem] flex-col bg-background text-foreground">
      <div className="flex min-h-0 flex-1">
        <Sidebar />
        <div className="flex min-w-0 flex-1 flex-col">
          <AppHeader actions={<UncommittedIndicator />} />
          <main className="min-h-0 flex-1 overflow-auto">
            <QueryHighlight>
              <Outlet />
            </QueryHighlight>
          </main>
        </div>
      </div>
      <StatusBar />
    </div>
  )
}
