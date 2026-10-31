import type { Command } from '@/shared/lib'
import { appearances, setAppearance, useAppearance, type Appearance } from '../model/appearance'

const labels: Record<Appearance, { title: string; icon: string }> = {
  system: { title: 'Appearance: follow the system', icon: 'monitor' },
  light: { title: 'Appearance: light', icon: 'sun' },
  dark: { title: 'Appearance: dark', icon: 'moon' },
}

// Light, dark or the system's choice, remembered by this browser.
export function useAppearanceCommands(): Command[] {
  const current = useAppearance()
  return appearances.map((appearance) => ({
    id: `appearance:${appearance}`,
    group: 'Actions',
    title: labels[appearance].title,
    icon: labels[appearance].icon,
    keywords: ['theme', 'dark mode', 'light mode'],
    hint: appearance === current ? 'current' : undefined,
    run: () => setAppearance(appearance),
  }))
}
