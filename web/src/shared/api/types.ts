export type Category = 'backlog' | 'todo' | 'in_progress' | 'waiting' | 'blocked' | 'done' | 'dropped'

export type WaitingReason = 'review' | 'validation' | 'answer' | 'other'

export type Kind = 'task' | 'story' | 'reflexion' | 'ressource' | 'unknown'

export interface Item {
  node: string
  id: string | null
  reference: string | null
  title: string | null
  kind: Kind
  archived: boolean
  status: string | null
  category: Category | null
  size: string | null
  impact: string | null
  priority: string | null
  created: string | null
  type: string | null
  personas: string[]
  release: string | null
  milestone: string | null
  parent: string | null
  depends_on: string[]
  external: string[]
  branches: string[]
  pull_requests: string[]
  waiting_for: string[]
  waiting_on: WaitingReason | null
  waiting_since: string | null
  version: number | null
  path: string
  updated?: string | null
  body?: string
}

// One item found by `ariane search`: its row, where it matched, an excerpt and the ranges of the excerpt
// to highlight, in code points.
export interface SearchHit extends Item {
  field: 'reference' | 'title' | 'body'
  excerpt: string
  highlights: [number, number][]
}

export interface Relations {
  parent: Item | null
  children: Item[]
  depends_on: Item[]
  dependents: Item[]
  unresolved: string[]
}

export interface ItemDetail extends Item {
  header: Record<string, unknown> | null
  body: string
  relations: Relations
}

export type Severity = 'error' | 'warning'

export type Remedy = { kind: 'command'; command: string } | { kind: 'human'; resolutions: string[] }

export interface Finding {
  check: string
  severity: Severity
  node: string | null
  path: string | null
  message: string
  remedy: Remedy
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

// One person of a node's directory, `[people.<handle>]`: a name, and ids in trackers such as `monday`.
export interface Person {
  name: string
  [tracker: string]: unknown
}

export interface Persona {
  name: string
}

export interface NodeSummary {
  path: string
  items: number
  workflow: WorkflowStatus[]
  people?: Record<string, Person>
  personas?: Record<string, Persona>
  tracker_from?: string | null
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

export type EditableField = 'body' | 'status' | 'size' | 'impact' | 'priority' | 'waiting_for' | 'waiting_on' | 'waiting_since'

export type FieldValue = string | string[] | null

export interface FieldChange {
  from: FieldValue
  to: FieldValue
}

export type ItemChanges = Partial<Record<EditableField, FieldChange>>

export interface ItemPatch {
  path: string
  changes: ItemChanges
}

export interface PersonEntry {
  handle: string
  name: string
  monday: number | null
  awaited_by: string[]
}

// A node that declares a tracker or people, with its people.
export interface Directory {
  node: string
  tracker: string | null
  account: string | null
  people: PersonEntry[]
}

export interface PersonChange {
  node: string
  handle: string
  name?: string
  monday?: number | null
  rename_to?: string
}

export interface MondayUser {
  id: number
  name: string
  email: string | null
}

export type SyncAction =
  | { action: 'in_step' }
  | { action: 'push'; from: string | null; to: string; mention: [string, number][] }
  | { action: 'conflict'; monday: string | null; last: string | null }
  | { action: 'refused'; reason: string }

export type SyncStep = { item: string; ticket: number; category: Category } & SyncAction

export interface SyncReport {
  applied: boolean
  steps: SyncStep[]
}

export interface Dashboard {
  pinned: string[]
}

export interface Theme {
  primary: string
  secondary: string
}

export interface Settings {
  version: number
  theme: Theme
  dashboard: Dashboard
}

export interface CommitResult {
  commit: string
}

export interface PullResult {
  pulled: number
}

export interface DiscardResult {
  discarded: string[]
}

export interface FileDiff {
  path: string
  diff: string
}
