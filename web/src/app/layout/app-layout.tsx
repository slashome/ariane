import { Outlet } from 'react-router'
import { AppHeader } from '@/widgets/app-header'
import { Sidebar } from '@/widgets/sidebar'
import { UncommittedIndicator } from '@/widgets/uncommitted-indicator'

export function AppLayout() {
  return (
    <div className="flex h-screen min-w-[64rem] bg-background text-foreground">
      <Sidebar />
      <div className="flex min-w-0 flex-1 flex-col">
        <AppHeader actions={<UncommittedIndicator />} />
        <main className="min-h-0 flex-1 overflow-auto">
          <Outlet />
        </main>
      </div>
    </div>
  )
}
