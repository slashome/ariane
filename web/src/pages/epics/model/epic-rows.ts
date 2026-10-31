import { epicProgress, isClosed, sortByManualOrder, type EpicProgress } from '@/entities/item'
import type { Item } from '@/shared/api'

export interface EpicRow extends EpicProgress {
  epic: Item
}

// Each epic with its stories and their tasks, in the manual order of their node: live epics first, closed ones after.
export function epicRows(epics: Item[], items: Item[]): EpicRow[] {
  const rows = sortByManualOrder(
    epics.map((epic) => ({ epic, ...epicProgress(epic, items) })),
    (row) => row.epic,
  )
  return [...rows.filter((row) => !isClosed(row.epic.category)), ...rows.filter((row) => isClosed(row.epic.category))]
}
