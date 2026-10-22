import type { Category, EditableField, Item, ItemChanges, Kind } from '@/shared/api'

export const categoryOrder: Category[] = ['backlog', 'todo', 'in_progress', 'blocked', 'done', 'dropped']

const workKinds: Kind[] = ['task', 'story']

export function isClosed(category: Category | null): boolean {
  return category === 'done' || category === 'dropped'
}

export function isWorkItem(item: Item): boolean {
  return workKinds.includes(item.kind)
}

export function changesFor(item: Item, next: Partial<Record<EditableField, string | null>>): ItemChanges {
  const changes: ItemChanges = {}
  for (const field of Object.keys(next) as EditableField[]) {
    const to = next[field] ?? null
    if (item[field] !== to) {
      changes[field] = { from: item[field], to }
    }
  }
  return changes
}
