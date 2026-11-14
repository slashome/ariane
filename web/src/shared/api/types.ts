export type Category = 'backlog' | 'todo' | 'in_progress' | 'waiting' | 'blocked' | 'done' | 'dropped'

export type WaitingReason = 'review' | 'validation' | 'answer' | 'other'

export type Kind = 'task' | 'epic' | 'story' | 'reflexion' | 'ressource' | 'unknown'

export type ReleaseSource = 'own' | 'story' | 'epic'

export interface EffectiveRelease {
  name: string
  source: ReleaseSource
  from: string | null
  position: number | null
  parking: boolean
}

export type EstimateBasis = 'size' | 'provisional' | 'default' | 'tasks'

export type Tester = 'owner' | 'others'

export interface Estimate {
  hours: number
  remaining_hours: number
  test_hours: number
  remaining_test_hours: number
  tester: Tester
  basis: EstimateBasis
  size: string | null
  parts: number
  guessed: number
}

export interface Pricing {
  XS: number
  S: number
  M: number
  L: number
  XL: number
  default: string
  test_ratio: number
  tester: Tester
}

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
  estimate: Estimate | null
  impact: string | null
  priority: string | null
  rank: number | null
  created: string | null
  closed: string | null
  type: string | null
  personas: string[]
  release: string | null
  effective_release: EffectiveRelease | null
  parent: string | null
  // The epic of a story, or of a task's story, as `<node>#<id>`.
  epic: string | null
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
  closed_approx?: string | null
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

export type WorkflowStage = 'testing'

export interface WorkflowStatus {
  id: string
  label: string
  category: Category
  stage?: WorkflowStage
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

export interface Release {
  name: string
  parking: boolean
  target: string | null
}

export type ReleasePlace = 'before' | 'after' | 'top' | 'bottom'

export interface ReleaseChange {
  node: string
  name: string
  place?: ReleasePlace
  other?: string
  parking?: boolean
  target?: string | null
  rename_to?: string
}

export interface ReleaseList {
  node: string
  releases_from: string | null
  releases: Release[]
  rewritten: string[]
}

export interface NodeSummary {
  path: string
  description?: string | null
  items: number
  workflow: WorkflowStatus[]
  releases?: Release[] | null
  releases_from?: string | null
  estimates?: Pricing
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

export type EditableField =
  | 'body'
  | 'status'
  | 'size'
  | 'impact'
  | 'priority'
  | 'release'
  | 'waiting_for'
  | 'waiting_on'
  | 'waiting_since'

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

export type Place = 'before' | 'after' | 'top' | 'bottom' | 'clear'

export interface RankRequest {
  path: string
  place: Place
  other?: string
  seen: Record<string, number | null>
  status?: FieldChange
}

export interface RankResult {
  items: Item[]
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

export interface PersonaEntry {
  id: string
  name: string
  used_by: string[]
}

// A node that declares a tracker or personas, with its personas.
export interface PersonaDirectory {
  node: string
  personas: PersonaEntry[]
}

export interface PersonaChange {
  node: string
  id: string
  name?: string
  rename_to?: string
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
  | { action: 'in_step'; label: string }
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

export interface ProxyCheck {
  ok: boolean
  what: string
  hint: string
}

export interface ProxyRoute {
  host: string
  port: number
  node: string | null
  declared: boolean
  listening: boolean
}

export interface ProxyReport {
  ok: boolean
  proxy_port: number
  server_port: number
  checks: ProxyCheck[]
  routes: ProxyRoute[]
  problems: string[]
}

export interface RouteTable {
  routes: { host: string; port: number; node: string | null }[]
  problems: string[]
}

export interface RouteChange {
  node: string
  host: string
  port: number
  replaces?: string
}

export interface RouteRemoval {
  node: string
  host: string
}

export interface Moment {
  hours: number
  at: string | null
}

export interface Span {
  start: Moment
  end: Moment
}

export interface DateRange {
  from: string
  to: string
}

export interface ClosedDate {
  date: string
  approximate: boolean
}

export interface ForecastSummary {
  hours: number
  test_hours: number
  span: Span | null
  closed: DateRange | null
  done: number
  open: number
}

export type WorkStage = 'open' | 'in_progress' | 'testing' | 'waiting' | 'done'

export type SegmentPhase = 'work' | 'test'

export interface Segment {
  phase: SegmentPhase
  start: Moment
  end: Moment
}

export interface WorkPlan {
  reference: string
  node: string
  kind: Kind
  title: string
  stage: WorkStage
  hours: number
  test_hours: number
  tester: Tester
  basis: EstimateBasis
  target: string | null
  sequence: number | null
  start: Moment | null
  end: Moment | null
  delivered: Moment | null
  segments: Segment[]
  validation: Span | null
  closed: ClosedDate | null
}

export interface StoryPlan extends ForecastSummary {
  reference: string | null
  title: string | null
  target: string | null
  work: WorkPlan[]
}

export interface EpicPlan extends ForecastSummary {
  reference: string | null
  title: string | null
  target: string | null
  stories: StoryPlan[]
}

export interface ReleasePlan extends ForecastSummary {
  name: string
  declared_by: string
  target: string | null
  epics: EpicPlan[]
}

export type UnscheduledReason = 'parking' | 'no_release' | 'undeclared_release' | 'cycle' | 'blocked_by_cycle'

export interface Unscheduled {
  reference: string
  node: string
  kind: Kind
  title: string
  reason: UnscheduledReason
  release: string | null
}

export type ForecastWarningKind =
  | 'unsized'
  | 'provisional'
  | 'without_story'
  | 'without_epic'
  | 'without_release'
  | 'undeclared_release'
  | 'missed_target'
  | 'contradictory_target'
  | 'dependency_cycle'
  | 'unscheduled_dependency'

export interface FlaggedItem {
  reference: string
  detail: string | null
}

export interface ForecastWarning {
  kind: ForecastWarningKind
  count: number
  items: FlaggedItem[]
}

export interface Roadmap {
  node: string
  carrier: string | null
  timezone: string | null
  now: string
  dated: boolean
  hours: number
  test_hours: number
  releases: ReleasePlan[]
  unscheduled: Unscheduled[]
  warnings: ForecastWarning[]
}
