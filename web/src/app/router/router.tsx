import { createBrowserRouter, Navigate, useLocation, type RouteObject } from 'react-router'
import { BacklogPage } from '@/pages/backlog'
import { DashboardPage } from '@/pages/dashboard'
import { EpicPage } from '@/pages/epic'
import { EpicsPage } from '@/pages/epics'
import { HealthPage } from '@/pages/health'
import { ItemPage } from '@/pages/item'
import { KanbanPage } from '@/pages/kanban'
import { MatrixPage } from '@/pages/matrix'
import { JournalPage, ReflexionsPage } from '@/pages/reflexions'
import { PeoplePage } from '@/pages/people'
import { ProxyPage } from '@/pages/proxy'
import { RepoPage } from '@/pages/repo'
import { RessourcesPage } from '@/pages/ressources'
import { SettingsPage } from '@/pages/settings'
import { StoriesPage } from '@/pages/stories'
import { StoryPage } from '@/pages/story'
import { TasksPage } from '@/pages/tasks'
import { routes } from '@/shared/config'
import { AppLayout } from '../layout/app-layout'

// The addresses of the first interface, kept working: they land on their new place with their filters.
function Moved({ to }: { to: string }) {
  const { search } = useLocation()
  return <Navigate to={{ pathname: to, search }} replace />
}

export const routeTree: RouteObject[] = [
  {
    element: <AppLayout />,
    children: [
      { index: true, element: <DashboardPage /> },
      { path: 'dashboard', element: <Moved to={routes.home} /> },
      { path: 'kanban', element: <Moved to={routes.kanban} /> },
      { path: 'matrix', element: <Moved to={routes.matrix} /> },
      { path: 'tasks', element: <Moved to={routes.taskList} /> },
      { path: 'tasks/list', element: <TasksPage /> },
      { path: 'tasks/kanban', element: <KanbanPage /> },
      { path: 'tasks/matrix', element: <MatrixPage /> },
      { path: 'tasks/backlog', element: <BacklogPage /> },
      { path: 'tasks/*', element: <ItemPage sectionLabel="Tasks" /> },
      { path: 'epics', element: <EpicsPage /> },
      { path: 'epics/*', element: <EpicPage /> },
      { path: 'stories', element: <StoriesPage /> },
      { path: 'stories/*', element: <StoryPage /> },
      { path: 'reflexions', element: <ReflexionsPage /> },
      { path: 'reflexions/journal', element: <JournalPage /> },
      { path: 'reflexions/*', element: <ItemPage sectionLabel="Reflexions" /> },
      { path: 'ressources', element: <RessourcesPage /> },
      { path: 'ressources/*', element: <ItemPage sectionLabel="Ressources" /> },
      { path: 'health', element: <HealthPage /> },
      { path: 'repo', element: <RepoPage /> },
      { path: 'proxy', element: <ProxyPage /> },
      { path: 'settings', element: <SettingsPage /> },
      { path: 'people', element: <PeoplePage /> },
      { path: '*', element: <Navigate to="/" replace /> },
    ],
  },
]

export const router = createBrowserRouter(routeTree)
