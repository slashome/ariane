import { createBrowserRouter, Navigate } from 'react-router'
import { DashboardPage } from '@/pages/dashboard'
import { KanbanPage } from '@/pages/kanban'
import { MatrixPage } from '@/pages/matrix'
import { TasksPage } from '@/pages/tasks'
import { AppLayout } from '../layout/app-layout'

export const router = createBrowserRouter([
  {
    element: <AppLayout />,
    children: [
      { index: true, element: <Navigate to="/tasks" replace /> },
      { path: 'dashboard', element: <DashboardPage /> },
      { path: 'tasks', element: <TasksPage /> },
      { path: 'kanban', element: <KanbanPage /> },
      { path: 'matrix', element: <MatrixPage /> },
      { path: '*', element: <Navigate to="/tasks" replace /> },
    ],
  },
])
