import { useEffect, useMemo, useState, type KeyboardEvent } from 'react'
import type { Command } from '@/shared/lib'
import { rankCommands, type Entry } from './rank'
import { currentPlatform, isApple, isPaletteShortcut } from './shortcut'

const perSection = 8

// Whether the palette is open; ⌘K on an Apple system, Ctrl+K elsewhere, toggles it from anywhere.
export function usePaletteToggle() {
  const [open, setOpen] = useState(false)
  const apple = useMemo(() => isApple(currentPlatform()), [])
  useEffect(() => {
    const onKeyDown = (event: globalThis.KeyboardEvent) => {
      if (isPaletteShortcut(event, apple)) {
        event.preventDefault()
        setOpen((current) => !current)
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [apple])
  return { open, setOpen, shortcut: apple ? '⌘K' : 'Ctrl+K' }
}

// The list of an open palette: the sections found for the text, the stack of lists opened by a command
// with children, and the entry under the keyboard. Arrows (or Ctrl+N, Ctrl+P) move, Enter runs or
// opens, Shift+Enter runs the other way, Backspace on an empty text goes back up.
export function usePaletteList(all: Command[], text: string, setText: (text: string) => void, close: () => void) {
  const [stack, setStack] = useState<Command[]>([])
  const [active, setActive] = useState(0)
  const parent = stack[stack.length - 1]
  const sections = useMemo(() => rankCommands(parent?.children ?? all, text, parent ? Infinity : perSection), [parent, all, text])
  const entries = useMemo(() => sections.flatMap((section) => section.entries), [sections])
  const current = Math.min(active, Math.max(entries.length - 1, 0))

  const choose = (entry: Entry, shift: boolean) => {
    const { command } = entry
    if (command.children) {
      setStack([...stack, command])
      setText('')
      setActive(0)
      return
    }
    close()
    command.run?.({ shift })
  }

  const back = () => {
    setStack(stack.slice(0, -1))
    setText('')
    setActive(0)
  }

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    const move = (to: number) => {
      event.preventDefault()
      if (entries.length > 0) {
        setActive((to + entries.length) % entries.length)
      }
    }
    if (event.key === 'ArrowDown' || (event.ctrlKey && event.key === 'n')) {
      move(current + 1)
    } else if (event.key === 'ArrowUp' || (event.ctrlKey && event.key === 'p')) {
      move(current - 1)
    } else if (event.key === 'PageDown') {
      move(Math.min(current + 5, entries.length - 1))
    } else if (event.key === 'PageUp') {
      move(Math.max(current - 5, 0))
    } else if (event.key === 'Enter') {
      event.preventDefault()
      const entry = entries[current]
      if (entry !== undefined) {
        choose(entry, event.shiftKey)
      }
    } else if (event.key === 'Backspace' && text === '' && stack.length > 0) {
      event.preventDefault()
      back()
    }
  }

  return {
    stack,
    back,
    sections,
    entries,
    active: entries[current],
    setActive: (entry: Entry) => setActive(entries.indexOf(entry)),
    choose,
    onKeyDown,
    onTextChange: (value: string) => {
      setText(value)
      setActive(0)
    },
  }
}
