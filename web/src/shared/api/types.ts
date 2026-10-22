export type Category = 'backlog' | 'todo' | 'in_progress' | 'blocked' | 'done' | 'dropped'

export type Kind = 'backlog' | 'task' | 'story' | 'reflexion' | 'ressource' | 'unknown'

export interface Item {
  node: string
  id: string | null
  title: string | null
  kind: Kind
  archived: boolean
  status: string | null
  category: Category | null
  size: string | null
  impact: string | null
  priority: string | null
  created: string | null
  external: string[]
  version: number | null
  path: string
}

export interface WorkflowStatus {
  id: string
  label: string
  category: Category
}

export interface Tracker {
  kind?: string
  account?: string
  board?: number | string
  [key: string]: unknown
}

export interface NodeSummary {
  path: string
  has_index: boolean
  items: number
  workflow: WorkflowStatus[]
  tracker?: Tracker | null
  last_change?: string | null
}

export interface SchemaKey {
  name: string
  type: string
  values?: string[]
}

export interface Schema {
  version: number
  categories: { id: Category; label: string }[]
  id_prefixes: Record<Kind, string>
  keys: SchemaKey[]
  reflexion_statuses: string[]
}

export interface UncommittedFile {
  code: string
  path: string
}

export interface Repo {
  root: string
  branch: string | null
  ahead: number
  uncommitted: UncommittedFile[]
}

export type EditableField = 'status' | 'size' | 'impact' | 'priority'

export interface FieldChange {
  from: string | null
  to: string | null
}

export type ItemChanges = Partial<Record<EditableField, FieldChange>>

export interface ItemPatch {
  path: string
  changes: ItemChanges
}

export interface Dashboard {
  pinned: string[]
}

export interface CommitResult {
  commit: string
}

export interface DiscardResult {
  discarded: string[]
}

export interface FileDiff {
  path: string
  diff: string
}
