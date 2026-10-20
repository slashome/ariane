import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { RouterProvider } from 'react-router/dom'
import { AppProviders } from './data/app-providers'
import { router } from './router/router'
import './styles/index.css'

const root = document.getElementById('root')
if (!root) {
  throw new Error('missing #root element')
}

createRoot(root).render(
  <StrictMode>
    <AppProviders>
      <RouterProvider router={router} />
    </AppProviders>
  </StrictMode>,
)
