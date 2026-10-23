import { Eye, Pin, RotateCcw, Save, TriangleAlert } from 'lucide-react'
import { useEffect, useState } from 'react'
import { defaultColor, previewTheme, readableOnLight, useSaveTheme, useSettings } from '@/entities/theme'
import type { Theme } from '@/shared/api'
import { cn } from '@/shared/lib'
import { Brand, Button, ErrorState, LoadingState, PageHeader } from '@/shared/ui'
import { presets, sameTheme } from '../model/presets'
import { ColorField } from './color-field'

function Preview() {
  return (
    <div className="space-y-3 rounded-lg border border-border p-4" aria-label="Preview">
      <div className="flex items-center gap-2 font-semibold">
        <Brand />
      </div>
      <div className="flex flex-wrap items-center gap-3">
        <Button size="sm">Primary button</Button>
        <a href="#preview" onClick={(event) => event.preventDefault()} className="text-sm text-primary hover:underline">
          A link
        </a>
        <button type="button" className="icon-button size-6" aria-label="An icon button">
          <Eye className="size-3.5" aria-hidden />
        </button>
      </div>
      <div className="w-56 space-y-0.5">
        <div className="flex items-center gap-2.5 rounded-md bg-accent/15 py-1.5 pl-3 text-sm font-medium shadow-[inset_2px_0_0_var(--color-accent)]">
          <Pin className="size-4" aria-hidden />
          Active menu entry
        </div>
        <div className="flex items-center gap-2.5 rounded-md py-1.5 pl-3 text-sm text-muted-foreground">
          <Pin className="size-4" aria-hidden />
          Another entry
        </div>
      </div>
    </div>
  )
}

export function SettingsPage() {
  const { data: settings, isPending, error } = useSettings()
  const save = useSaveTheme()
  const [draft, setDraft] = useState<Theme | null>(null)
  const saved = settings?.theme
  const theme = draft ?? saved ?? { primary: defaultColor, secondary: defaultColor }
  const changed = draft !== null && !sameTheme(draft, saved)

  useEffect(() => {
    previewTheme(changed ? draft : null)
  }, [draft, changed])
  useEffect(() => () => previewTheme(null), [])

  const update = (next: Partial<Theme>) => setDraft({ ...theme, ...next })

  return (
    <div className="flex flex-col gap-6 p-6">
      <PageHeader
        title="Settings"
        description="Saved in settings.toml at the root of the content repository, versioned with the content: commit it to keep the theme on every machine."
      />
      {error ? (
        <ErrorState message={`Could not read the settings: ${error.message}`} />
      ) : isPending ? (
        <LoadingState rows={3} />
      ) : (
        <section aria-labelledby="theme-title" className="grid max-w-4xl gap-6 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
          <div className="space-y-5">
            <h2 id="theme-title" className="text-sm font-semibold">
              Theme
            </h2>
            <ColorField
              id="primary"
              label="Primary"
              hint="Buttons, links, focus rings, the logo and the favicon."
              value={theme.primary}
              onChange={(primary) => update({ primary })}
            />
            {!readableOnLight(theme.primary) && (
              <p className="flex items-start gap-1.5 text-xs text-amber-700 dark:text-amber-300">
                <TriangleAlert className="mt-px size-3.5 shrink-0" aria-hidden />
                Too light to stand out from the light background (contrast under 3:1): links and focus rings will be hard to see.
              </p>
            )}
            <ColorField
              id="secondary"
              label="Secondary"
              hint="The accent: the active menu entry and the halo behind icon buttons."
              value={theme.secondary}
              onChange={(secondary) => update({ secondary })}
            />
            <div className="space-y-1.5">
              <p className="text-sm font-medium">Presets</p>
              <ul className="flex flex-wrap gap-2">
                {presets.map((preset) => (
                  <li key={preset.name}>
                    <button
                      type="button"
                      aria-pressed={sameTheme(preset.theme, theme)}
                      onClick={() => setDraft(preset.theme)}
                      className={cn(
                        'flex items-center gap-2 rounded-full border border-border py-1 pr-3 pl-1 text-xs hover:bg-muted',
                        sameTheme(preset.theme, theme) && 'border-primary',
                      )}
                    >
                      <span className="flex" aria-hidden>
                        <span className="size-4 rounded-full" style={{ background: preset.theme.primary }} />
                        <span className="-ml-1 size-4 rounded-full ring-2 ring-background" style={{ background: preset.theme.secondary }} />
                      </span>
                      {preset.name}
                    </button>
                  </li>
                ))}
              </ul>
            </div>
            <div className="flex flex-wrap gap-2 pt-1">
              <Button disabled={!changed || save.isPending} onClick={() => draft && save.mutate(draft, { onSuccess: () => setDraft(null) })}>
                <Save className="size-4" aria-hidden />
                {save.isPending ? 'Saving…' : 'Save'}
              </Button>
              <Button variant="ghost" disabled={!changed} onClick={() => setDraft(null)}>
                Cancel
              </Button>
              <Button variant="outline" className="ml-auto" onClick={() => setDraft({ primary: defaultColor, secondary: defaultColor })}>
                <RotateCcw className="size-4" aria-hidden />
                Default
              </Button>
            </div>
            {changed && <p className="text-xs text-muted-foreground">Previewing across the interface — not saved yet.</p>}
          </div>
          <Preview />
        </section>
      )}
    </div>
  )
}
