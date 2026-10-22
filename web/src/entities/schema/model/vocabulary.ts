import type { Category, Kind, Schema } from '@/shared/api'

export interface VocabularyOption<T extends string = string> {
  value: T
  label: string
}

const kindLabels: Record<Kind, string> = {
  task: 'Task',
  story: 'Story',
  reflexion: 'Reflexion',
  ressource: 'Ressource',
  unknown: 'Unknown',
}

export function categoryOptions(schema: Schema | undefined): VocabularyOption<Category>[] {
  return (schema?.categories ?? []).map(({ id, label }) => ({ value: id, label }))
}

export function kindOptions(schema: Schema | undefined): VocabularyOption<Kind>[] {
  return (Object.keys(schema?.id_prefixes ?? {}) as Kind[]).map((kind) => ({
    value: kind,
    label: kindLabels[kind],
  }))
}

export function kindLabel(kind: Kind): string {
  return kindLabels[kind]
}

export function enumValues(schema: Schema | undefined, key: string): string[] {
  return schema?.keys.find((entry) => entry.name === key)?.values ?? []
}
