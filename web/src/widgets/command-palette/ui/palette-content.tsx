import { Dialog as DialogPrimitive } from 'radix-ui'
import { useId, useState } from 'react'
import { Icon, Kbd } from '@/shared/ui'
import { usePaletteCommands } from '../model/use-palette-commands'
import { usePaletteList } from '../model/use-palette'
import { PaletteOption } from './palette-option'
import './command-palette.css'

// The open palette: a combobox over a listbox of sections, run from the keyboard. Mounted only while
// open, so it starts afresh every time.
export function PaletteContent({ close }: { close: () => void }) {
  const [text, setText] = useState('')
  const { commands, searching } = usePaletteCommands(text)
  const list = usePaletteList(commands, text, setText, close)
  const id = useId()
  const listbox = `${id}-list`
  const optionId = (index: number) => `${id}-option-${index}`
  const activeIndex = list.active === undefined ? -1 : list.entries.indexOf(list.active)
  const parent = list.stack[list.stack.length - 1]

  return (
    <DialogPrimitive.Content
      aria-describedby={undefined}
      onEscapeKeyDown={(event) => {
        if (list.stack.length > 0) {
          event.preventDefault()
          list.back()
        }
      }}
      className="command-palette"
    >
      <DialogPrimitive.Title className="lu-visually-hidden">Command palette</DialogPrimitive.Title>
      <div className="command-palette__search">
        {parent ? (
          <button type="button" onClick={list.back} className="lu-icon-button command-palette__back" aria-label={`Back from ${parent.title}`}>
            <Icon name="chevron-left" className="command-palette__icon" />
          </button>
        ) : (
          <Icon name="command" className="command-palette__search-icon" />
        )}
        {parent && <span className="command-palette__parent">{parent.title.replace(/…$/, '')}</span>}
        <input
          autoFocus
          role="combobox"
          aria-label={parent ? parent.title : 'Search items, views, nodes and actions'}
          aria-autocomplete="list"
          aria-expanded="true"
          aria-controls={listbox}
          aria-activedescendant={activeIndex === -1 ? undefined : optionId(activeIndex)}
          autoComplete="off"
          spellCheck={false}
          placeholder={parent ? 'Filter…' : 'Search items, views, nodes, actions, text…'}
          value={text}
          onChange={(event) => list.onTextChange(event.target.value)}
          onKeyDown={list.onKeyDown}
          className="command-palette__input"
        />
        {searching && text.trim().length >= 2 && (
          <span role="status" aria-label="Searching the text" className="command-palette__searching">
            <Icon name="loading" className="command-palette__icon lu-spin" />
          </span>
        )}
      </div>
      <ul id={listbox} role="listbox" aria-label="Results" className="command-palette__list">
        {list.sections.map((section, sectionIndex) => (
          <li key={section.group} role="presentation" className="command-palette__section">
            <ul role="group" aria-labelledby={`${id}-section-${sectionIndex}`}>
              <li role="presentation" id={`${id}-section-${sectionIndex}`} className="command-palette__group">
                {section.group}
              </li>
              {section.entries.map((entry) => {
                const position = list.entries.indexOf(entry)
                return (
                  <PaletteOption
                    key={entry.command.id}
                    id={optionId(position)}
                    entry={entry}
                    active={position === activeIndex}
                    onHover={() => list.setActive(entry)}
                    onChoose={(shift) => list.choose(entry, shift)}
                  />
                )
              })}
            </ul>
          </li>
        ))}
        {list.entries.length === 0 && (
          <li role="presentation" className="command-palette__empty">
            {searching ? 'Searching…' : text.trim() === '' ? 'Nothing to list here.' : `No result for “${text.trim()}”.`}
          </li>
        )}
      </ul>
      <div className="command-palette__footer">
        <span>
          <Kbd>↑</Kbd> <Kbd>↓</Kbd> move
        </span>
        <span>
          <Kbd>↵</Kbd> {list.active?.command.children ? 'open' : 'run'}
        </span>
        {list.active?.command.shiftLabel && (
          <span>
            <Kbd>⇧</Kbd> <Kbd>↵</Kbd> {list.active.command.shiftLabel}
          </span>
        )}
        <span className="command-palette__escape">
          <Kbd>esc</Kbd> {parent ? 'back' : 'close'}
        </span>
      </div>
    </DialogPrimitive.Content>
  )
}
