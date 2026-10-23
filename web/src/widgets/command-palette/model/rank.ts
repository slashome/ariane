import { foldText, highlightRanges, type Command, type Range } from '@/shared/lib'

export interface Entry {
  command: Command
  // The ranges of the title to highlight.
  ranges: readonly Range[]
}

export interface Section {
  group: string
  entries: Entry[]
}

// How well a command answers the words, lower first; `null` when a word is in neither its title nor its
// keywords. An id or reference equal to the whole text comes first, then a title that starts with it,
// then titles whose words start with the words typed, then any other match.
function score(command: Command, words: string[], whole: string): number | null {
  const title = foldText(command.title)
  const keywords = (command.keywords ?? []).map(foldText)
  if (!words.every((word) => title.includes(word) || keywords.some((keyword) => keyword.includes(word)))) {
    return null
  }
  if (keywords.includes(whole)) {
    return 0
  }
  if (title.startsWith(whole)) {
    return 1
  }
  const starts = title.split(/[\s\-_/:.,—–'’()]+/)
  if (words.every((word) => starts.some((start) => start.startsWith(word)))) {
    return 2
  }
  return words.every((word) => title.includes(word)) ? 3 : 4
}

// The commands to list for a text, in sections in the order their groups first appear: without text,
// those not kept for searching; with text, those that match, best first within a section, at most
// `limit` per section. Commands found by their provider keep their order and their place.
export function rankCommands(commands: Command[], text: string, limit = Infinity): Section[] {
  const whole = foldText(text.trim())
  const words = whole.split(/\s+/).filter((word) => word !== '')
  const ranked: (Entry & { score: number; index: number })[] = []
  commands.forEach((command, index) => {
    if (command.matched) {
      ranked.push({ command, ranges: [], score: 0, index })
      return
    }
    if (words.length === 0) {
      if (!command.searchOnly) {
        ranked.push({ command, ranges: [], score: 0, index })
      }
      return
    }
    const found = score(command, words, whole)
    if (found !== null) {
      ranked.push({ command, ranges: highlightRanges(command.title, words), score: found, index })
    }
  })
  const sections = new Map<string, (Entry & { score: number; index: number })[]>()
  for (const entry of ranked) {
    sections.set(entry.command.group, [...(sections.get(entry.command.group) ?? []), entry])
  }
  return [...sections].map(([group, entries]) => ({
    group,
    entries: entries
      .sort((left, right) => left.score - right.score || left.index - right.index)
      .slice(0, limit)
      .map(({ command, ranges }) => ({ command, ranges })),
  }))
}
