import type { LucideIcon } from 'lucide-react'
import type { Range } from './highlight'

// One entry of the command palette. Each slice builds its own; the palette only finds, shows and runs them.
export interface Command {
  // Unique across the palette.
  id: string
  // The section the command is listed under.
  group: string
  title: string
  // Shown at the right: an id, a path, a state.
  hint?: string
  // Words that find the command besides its title, such as an id or a reference; one equal to the whole
  // text typed ranks the command first.
  keywords?: string[]
  icon?: LucideIcon
  // A line under the title with the ranges to highlight, such as an excerpt found by `ariane search`.
  detail?: { text: string; ranges: readonly Range[] }
  // Listed only once something is typed: a long list, such as the items, that would drown the rest.
  searchOnly?: boolean
  // Already found by its provider for the text typed: kept in its order, never filtered again.
  matched?: boolean
  // What Enter does; `shift` is set by Shift+Enter, whose meaning `shiftLabel` names when it has one.
  run?: (how: { shift: boolean }) => void
  shiftLabel?: string
  // A command that opens a list of its own instead of running.
  children?: Command[]
}
