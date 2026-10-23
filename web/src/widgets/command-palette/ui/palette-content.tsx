import { Dialog as DialogPrimitive } from 'radix-ui'
import { ChevronLeft, Command as CommandIcon, Loader2 } from 'lucide-react'
import { useId, useState } from 'react'
import { Kbd } from '@/shared/ui'
import { usePaletteCommands } from '../model/use-palette-commands'
import { usePaletteList } from '../model/use-palette'
import { PaletteOption } from './palette-option'

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
      className="fixed top-[12vh] left-1/2 z-50 flex max-h-[70vh] w-[40rem] max-w-[calc(100%-2rem)] -translate-x-1/2 flex-col overflow-hidden rounded-xl border border-border bg-popover text-foreground shadow-2xl focus:outline-none"
    >
      <DialogPrimitive.Title className="sr-only">Command palette</DialogPrimitive.Title>
      <div className="flex items-center gap-2 border-b border-border px-3">
        {parent ? (
          <button type="button" onClick={list.back} className="icon-button size-6" aria-label={`Back from ${parent.title}`}>
            <ChevronLeft className="size-4" aria-hidden />
          </button>
        ) : (
          <CommandIcon className="size-4 shrink-0 text-muted-foreground" aria-hidden />
        )}
        {parent && <span className="shrink-0 rounded bg-muted px-1.5 py-0.5 text-xs text-muted-foreground">{parent.title.replace(/…$/, '')}</span>}
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
          className="h-12 min-w-0 flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground"
        />
        {searching && text.trim().length >= 2 && (
          <span role="status" aria-label="Searching the text" className="text-muted-foreground">
            <Loader2 className="size-4 animate-spin" aria-hidden />
          </span>
        )}
      </div>
      <ul id={listbox} role="listbox" aria-label="Results" className="min-h-0 flex-1 overflow-y-auto p-2">
        {list.sections.map((section, sectionIndex) => (
          <li key={section.group} role="presentation" className="mb-1">
            <ul role="group" aria-labelledby={`${id}-section-${sectionIndex}`}>
              <li role="presentation" id={`${id}-section-${sectionIndex}`} className="px-3 pt-2 pb-1 text-[11px] font-semibold tracking-wide text-muted-foreground uppercase">
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
          <li role="presentation" className="px-3 py-10 text-center text-sm text-muted-foreground">
            {searching ? 'Searching…' : text.trim() === '' ? 'Nothing to list here.' : `No result for “${text.trim()}”.`}
          </li>
        )}
      </ul>
      <div className="flex flex-wrap items-center gap-x-4 gap-y-1 border-t border-border bg-muted/40 px-3 py-2 text-[11px] text-muted-foreground">
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
        <span className="ml-auto">
          <Kbd>esc</Kbd> {parent ? 'back' : 'close'}
        </span>
      </div>
    </DialogPrimitive.Content>
  )
}
