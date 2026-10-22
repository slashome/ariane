import type { Theme } from '@/shared/api'
import { faviconHref } from './favicon'

// Puts the theme on the document: the two `--theme-*` variables every color derives from, and the
// favicon in the primary color.
export function applyTheme(theme: Theme, document: Document = window.document) {
  const root = document.documentElement
  root.style.setProperty('--theme-primary', theme.primary)
  root.style.setProperty('--theme-secondary', theme.secondary)
  let icon = document.querySelector<HTMLLinkElement>('link[rel="icon"]')
  if (icon === null) {
    icon = document.createElement('link')
    icon.rel = 'icon'
    document.head.appendChild(icon)
  }
  icon.type = 'image/svg+xml'
  icon.href = faviconHref(theme.primary)
}
