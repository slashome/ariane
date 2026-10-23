import { useFullTextCommands, useItemCommands } from '@/entities/item'
import { useAppearanceCommands } from '@/entities/theme'
import { useFilterCommands, useItemFilters } from '@/features/filter-items'
import { useViewCommands } from './use-view-commands'
import { useSyncCommands } from '@/features/sync-item'
import { useDebouncedValue, type Command } from '@/shared/lib'

// Every command the palette offers for a text, each list from the slice that owns it: the items, the
// views, the nodes and the actions, then the items whose body holds the text, from `ariane search`.
export function usePaletteCommands(text: string): { commands: Command[]; searching: boolean } {
  const { filters } = useItemFilters()
  const settled = useDebouncedValue(text, 200)
  const items = useItemCommands(filters.includeArchived)
  const views = useViewCommands()
  const filtersAndNodes = useFilterCommands()
  const appearance = useAppearanceCommands()
  const sync = useSyncCommands()
  const fullText = useFullTextCommands(settled, filters.includeArchived)
  const nodes = filtersAndNodes.filter((command) => command.group !== 'Actions')
  const toggles = filtersAndNodes.filter((command) => command.group === 'Actions')
  return {
    commands: [...items, ...views, ...nodes, ...toggles, ...appearance, ...sync, ...fullText.commands],
    searching: fullText.pending || settled !== text,
  }
}
