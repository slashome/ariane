import type { Theme } from '@/shared/api'

// A few pairs that read well in both modes; the first is Ariane's own.
export const presets: { name: string; theme: Theme }[] = [
  { name: 'Ariane', theme: { primary: '#3760c9', secondary: '#3760c9' } },
  { name: 'Indigo & amber', theme: { primary: '#4f46e5', secondary: '#d97706' } },
  { name: 'Teal & coral', theme: { primary: '#0f766e', secondary: '#e8604c' } },
  { name: 'Violet & cyan', theme: { primary: '#7c3aed', secondary: '#0891b2' } },
  { name: 'Forest & gold', theme: { primary: '#2f6b3b', secondary: '#b8860b' } },
  { name: 'Crimson & slate', theme: { primary: '#b42339', secondary: '#64748b' } },
]

export function sameTheme(left: Theme | undefined, right: Theme | undefined): boolean {
  return left?.primary.toLowerCase() === right?.primary.toLowerCase() && left?.secondary.toLowerCase() === right?.secondary.toLowerCase()
}
