import { useEffect, useState } from 'react'
import { defaultColor, previewTheme, readableOnLight, useSaveTheme, useSettings } from '@/entities/theme'
import type { Theme } from '@/shared/api'
import { cn } from '@/shared/lib'
import { Brand, Button, ErrorState, Icon, LoadingState, PageHeader } from '@/shared/ui'
import { presets, sameTheme } from '../model/presets'
import { ColorField } from './color-field'
import './settings.css'

function Preview() {
  return (
    <div className="theme-preview" aria-label="Preview">
      <div className="theme-preview__brand">
        <Brand />
      </div>
      <div className="theme-preview__controls">
        <Button size="sm">Primary button</Button>
        <a href="#preview" onClick={(event) => event.preventDefault()} className="theme-preview__link">
          A link
        </a>
        <button type="button" className="lu-icon-button theme-preview__icon-button" aria-label="An icon button">
          <Icon name="view" className="theme-preview__small-icon" />
        </button>
      </div>
      <div className="theme-preview__menu">
        <div className="theme-preview__entry theme-preview__entry--active">
          <Icon name="pin" className="theme-preview__icon" />
          Active menu entry
        </div>
        <div className="theme-preview__entry">
          <Icon name="pin" className="theme-preview__icon" />
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
    <div className="settings">
      <PageHeader
        title="Settings"
        description="Saved in settings.toml at the root of the content repository, versioned with the content: commit it to keep the theme on every machine."
      />
      {error ? (
        <ErrorState message={`Could not read the settings: ${error.message}`} />
      ) : isPending ? (
        <LoadingState rows={3} />
      ) : (
        <section aria-labelledby="theme-title" className="settings__theme">
          <div className="settings__form">
            <h2 id="theme-title" className="settings__title">
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
              <p className="settings__warning">
                <Icon name="warning" className="settings__warning-icon" />
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
            <div className="settings__presets">
              <p className="settings__label">Presets</p>
              <ul className="settings__preset-list">
                {presets.map((preset) => (
                  <li key={preset.name}>
                    <button
                      type="button"
                      aria-pressed={sameTheme(preset.theme, theme)}
                      onClick={() => setDraft(preset.theme)}
                      className={cn(
                        'preset-chip',
                        sameTheme(preset.theme, theme) && 'preset-chip--selected',
                      )}
                    >
                      <span className="preset-chip__swatches" aria-hidden>
                        <span className="preset-chip__swatch" style={{ background: preset.theme.primary }} />
                        <span className="preset-chip__swatch preset-chip__swatch--overlap" style={{ background: preset.theme.secondary }} />
                      </span>
                      {preset.name}
                    </button>
                  </li>
                ))}
              </ul>
            </div>
            <div className="settings__actions">
              <Button disabled={!changed || save.isPending} onClick={() => draft && save.mutate(draft, { onSuccess: () => setDraft(null) })}>
                <Icon name="save" className="settings__icon" />
                {save.isPending ? 'Saving…' : 'Save'}
              </Button>
              <Button variant="ghost" disabled={!changed} onClick={() => setDraft(null)}>
                Cancel
              </Button>
              <Button variant="outline" className="settings__default" onClick={() => setDraft({ primary: defaultColor, secondary: defaultColor })}>
                <Icon name="reset" className="settings__icon" />
                Default
              </Button>
            </div>
            {changed && <p className="settings__note">Previewing across the interface — not saved yet.</p>}
          </div>
          <Preview />
        </section>
      )}
    </div>
  )
}
