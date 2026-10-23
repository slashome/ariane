import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { RouterProvider } from 'react-router/dom'
import { applyAppearance, readAppearance, resolveAppearance, systemDark } from '@/entities/theme'
import { AppProviders } from './data/app-providers'
import { router } from './router/router'
import './styles/index.css'

const root = document.getElementById('root')
if (!root) {
  throw new Error('missing #root element')
}

// Before the first paint, so a dark page never flashes light.
applyAppearance(resolveAppearance(readAppearance(), systemDark.matches()))

createRoot(root).render(
  <StrictMode>
    <AppProviders>
      <RouterProvider router={router} />
    </AppProviders>
  </StrictMode>,
)
