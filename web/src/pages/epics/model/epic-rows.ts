import { categoryOrder, compareNatural, epicProgress, type EpicProgress } from '@/entities/item'
import type { Item } from '@/shared/api'

export interface EpicRow extends EpicProgress {
  epic: Item
}

const rank = (epic: Item) => (epic.category === null ? categoryOrder.length : categoryOrder.indexOf(epic.category))

// Each epic with its stories and their tasks, live epics first in workflow order, then by id.
export function epicRows(epics: Item[], items: Item[]): EpicRow[] {
  return epics
    .map((epic) => ({ epic, ...epicProgress(epic, items) }))
    .sort((left, right) => rank(left.epic) - rank(right.epic) || compareNatural(left.epic.id ?? '', right.epic.id ?? ''))
}
