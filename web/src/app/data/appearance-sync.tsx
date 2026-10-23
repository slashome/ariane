import { useEffect } from 'react'
import { applyAppearance, resolveAppearance, systemDark, useAppearance } from '@/entities/theme'

// Keeps the document light or dark: as chosen in this browser, or following the system as it changes.
export function AppearanceSync() {
  const appearance = useAppearance()
  useEffect(() => {
    const apply = () => applyAppearance(resolveAppearance(appearance, systemDark.matches()))
    apply()
    return appearance === 'system' ? systemDark.subscribe(apply) : undefined
  }, [appearance])
  return null
}
