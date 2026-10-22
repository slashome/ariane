import { useEffect } from 'react'
import { applyTheme, defaultColor, useSettings, useThemePreview } from '@/entities/theme'

// Keeps the document in the saved theme, or in the one being tried in Settings.
export function ThemeSync() {
  const { data: settings } = useSettings()
  const preview = useThemePreview()
  const primary = preview?.primary ?? settings?.theme.primary ?? defaultColor
  const secondary = preview?.secondary ?? settings?.theme.secondary ?? defaultColor
  useEffect(() => applyTheme({ primary, secondary }), [primary, secondary])
  return null
}
