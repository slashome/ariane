import type { Category, EditableField, FieldValue, Item, ItemChanges, Kind } from '@/shared/api'

export const categoryOrder: Category[] = ['backlog', 'todo', 'in_progress', 'waiting', 'blocked', 'done', 'dropped']

const workKinds: Kind[] = ['task', 'story']

export function isClosed(category: Category | null): boolean {
  return category === 'done' || category === 'dropped'
}

export function isWorkItem(item: Item): boolean {
  return workKinds.includes(item.kind)
}

// An empty list is no value: the server writes neither.
function normalized(value: FieldValue | undefined): FieldValue {
  return value === undefined || (Array.isArray(value) && value.length === 0) ? null : value
}

export function changesFor(item: Item, next: Partial<Record<EditableField, FieldValue>>): ItemChanges {
  const changes: ItemChanges = {}
  for (const field of Object.keys(next) as EditableField[]) {
    const from = normalized(item[field])
    const to = normalized(next[field])
    if (JSON.stringify(from) !== JSON.stringify(to)) {
      changes[field] = { from, to }
    }
  }
  return changes
}
