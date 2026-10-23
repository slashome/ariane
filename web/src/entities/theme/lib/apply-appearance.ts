// Puts light or dark on the document: `data-appearance`, which the styles and Tailwind's `dark:` read,
// and the color scheme of the browser's own controls.
export function applyAppearance(resolved: 'light' | 'dark', document: Document = window.document) {
  const root = document.documentElement
  root.dataset.appearance = resolved
  root.style.colorScheme = resolved
}

// Whether the system asks for dark, and a way to follow its changes.
export const systemDark = {
  matches: () => window.matchMedia('(prefers-color-scheme: dark)').matches,
  subscribe(listener: () => void) {
    const query = window.matchMedia('(prefers-color-scheme: dark)')
    query.addEventListener('change', listener)
    return () => query.removeEventListener('change', listener)
  },
}
