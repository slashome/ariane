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
  version: number | null
  path: string
}

export interface WorkflowStatus {
  id: string
  label: string
  category: Category
}

export interface NodeSummary {
  path: string
  has_index: boolean
  items: number
  workflow: WorkflowStatus[]
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
  uncommitted: UncommittedFile[]
}
