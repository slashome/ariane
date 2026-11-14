import { compareManualOrder, formatHours, isClosed } from '@/entities/item'
import type { EffectiveRelease, Item, Kind, NodeSummary, Release, Roadmap } from '@/shared/api'
import { foldText, isWithinNode } from '@/shared/lib'
import { formatMoment, hoursLabel } from './describe'
import { isHatched } from './rows'

export const planningKinds: Kind[] = ['epic', 'story', 'task']

export const backlogZone = 'backlog'

const releaseZonePrefix = 'release:'

export function releaseZone(name: string): string {
  return `${releaseZonePrefix}${name}`
}

export function zoneRelease(zone: string): string | null | undefined {
  if (zone === backlogZone) {
    return null
  }
  return zone.startsWith(releaseZonePrefix) ? zone.slice(releaseZonePrefix.length) : undefined
}

export interface ReleaseBoardList {
  declaredIn: string | null
  inherited: boolean
  releases: Release[]
}

export function boardList(nodes: NodeSummary[], node: string): ReleaseBoardList {
  const summary = nodes.find((entry) => entry.path === node)
  const declaredIn = summary?.releases_from ?? null
  return {
    declaredIn,
    inherited: declaredIn !== null && declaredIn !== node,
    releases: summary?.releases ?? [],
  }
}

export function displayOrder(releases: Release[]): Release[] {
  return [...releases.filter((release) => !release.parking), ...releases.filter((release) => release.parking)]
}

export function planningItems(items: Item[], nodes: NodeSummary[], node: string, declaredIn: string | null): Item[] {
  const listOf = new Map(nodes.map((entry) => [entry.path, entry.releases_from ?? null]))
  return items.filter(
    (item) =>
      !item.archived &&
      planningKinds.includes(item.kind) &&
      isWithinNode(item.node, node) &&
      (listOf.get(item.node) ?? null) === declaredIn,
  )
}

export type BacklogShow = 'unplanned' | 'all'

export interface BacklogFilters {
  show: BacklogShow
  kinds: Kind[]
  type: string | null
  text: string
}

export const defaultBacklogFilters: BacklogFilters = { show: 'unplanned', kinds: [], type: null, text: '' }

function matchesText(item: Item, text: string): boolean {
  const wanted = foldText(text.trim())
  return wanted === '' || foldText(`${item.id ?? ''} ${item.title ?? ''}`).includes(wanted)
}

export function backlogItems(items: Item[], filters: BacklogFilters): Item[] {
  return items.filter(
    (item) =>
      !isClosed(item.category) &&
      (filters.show === 'all' || item.effective_release === null) &&
      (filters.kinds.length === 0 || filters.kinds.includes(item.kind)) &&
      (filters.type === null || item.type === filters.type) &&
      matchesText(item, filters.text),
  )
}

export function typesOf(items: Item[]): string[] {
  return [...new Set(items.flatMap((item) => (item.type === null ? [] : [item.type])))].sort((left, right) =>
    left.localeCompare(right),
  )
}

export function columnItems(items: Item[], name: string): Item[] {
  return items.filter((item) => item.release === name && !isClosed(item.category))
}

export function releaseUsers(items: Item[], nodes: NodeSummary[], declaredIn: string | null, name: string): string[] {
  const listOf = new Map(nodes.map((entry) => [entry.path, entry.releases_from ?? null]))
  return items.flatMap((item) =>
    item.release === name && item.reference !== null && (listOf.get(item.node) ?? null) === declaredIn ? [item.reference] : [],
  )
}

export type OutlineEntry =
  | { type: 'item'; item: Item; depth: number }
  | { type: 'heading'; reference: string; label: string; depth: number }

function epicOf(item: Item): string | null {
  return item.kind === 'epic' ? item.reference : item.epic
}

function storyOf(item: Item): string | null {
  if (item.kind === 'story') {
    return item.reference
  }
  return item.kind === 'task' ? item.parent : null
}

function headingLabel(reference: string, known: ReadonlyMap<string, Item>): string {
  const item = known.get(reference)
  const id = reference.slice(reference.lastIndexOf('#') + 1)
  return item?.title ? `${id} ${item.title}` : id
}

function groupKeys<T>(entries: T[], keyOf: (entry: T) => string | null): (string | null)[] {
  const keys: (string | null)[] = []
  for (const entry of entries) {
    const key = keyOf(entry)
    if (!keys.includes(key)) {
      keys.push(key)
    }
  }
  return [...keys.filter((key) => key !== null), ...keys.filter((key) => key === null)]
}

export function outline(items: Item[], known: ReadonlyMap<string, Item>): OutlineEntry[] {
  const sorted = [...items].sort(compareManualOrder)
  const entries: OutlineEntry[] = []
  for (const epic of groupKeys(sorted, epicOf)) {
    const inEpic = sorted.filter((item) => epicOf(item) === epic)
    const epicItem = inEpic.find((item) => item.kind === 'epic')
    const storyDepth = epic === null ? 0 : 1
    if (epicItem) {
      entries.push({ type: 'item', item: epicItem, depth: 0 })
    } else if (epic !== null) {
      entries.push({ type: 'heading', reference: epic, label: headingLabel(epic, known), depth: 0 })
    }
    const below = inEpic.filter((item) => item.kind !== 'epic')
    for (const story of groupKeys(below, storyOf)) {
      const inStory = below.filter((item) => storyOf(item) === story)
      const storyItem = inStory.find((item) => item.kind === 'story')
      if (storyItem) {
        entries.push({ type: 'item', item: storyItem, depth: storyDepth })
      } else if (story !== null) {
        entries.push({ type: 'heading', reference: story, label: headingLabel(story, known), depth: storyDepth })
      }
      const taskDepth = story === null ? storyDepth : storyDepth + 1
      for (const task of inStory.filter((item) => item.kind === 'task')) {
        entries.push({ type: 'item', item: task, depth: taskDepth })
      }
    }
  }
  return entries
}

export interface ColumnSummary {
  hours: string
  end: string | null
  done: number
  open: number
}

export function columnSummary(roadmap: Roadmap | undefined, release: Release, cards: Item[]): ColumnSummary {
  const plan = roadmap?.releases.find((entry) => entry.name === release.name)
  if (!roadmap || !plan) {
    const hours = cards.reduce((sum, item) => sum + (item.estimate?.hours ?? 0), 0)
    const guessed = cards.some((item) => item.estimate === null || item.estimate.basis === 'default' || item.estimate.basis === 'provisional')
    return { hours: hours > 0 ? `${guessed ? '~' : ''}${formatHours(hours)}` : '', end: null, done: 0, open: cards.length }
  }
  const guessed = plan.epics.some((epic) => epic.stories.some((story) => story.work.some(isHatched)))
  const end = formatMoment(plan.span?.end ?? null, roadmap.dated)
  return {
    hours: hoursLabel(plan.hours, plan.test_hours, guessed),
    end: end === null ? null : roadmap.dated ? end.slice(0, 10) : end,
    done: plan.done,
    open: plan.open,
  }
}

export interface Assignment {
  release: string | null
  optimistic: Partial<Item>
}

function effectiveAfter(item: Item, release: string | null, releases: Release[]): EffectiveRelease | null {
  if (release === null) {
    return item.effective_release?.source === 'own' ? null : item.effective_release
  }
  const position = releases.findIndex((entry) => entry.name === release)
  return {
    name: release,
    source: 'own',
    from: null,
    position: position === -1 ? null : position,
    parking: releases[position]?.parking ?? false,
  }
}

export function assignment(item: Item, zone: string, releases: Release[]): Assignment | null {
  const release = zoneRelease(zone)
  if (release === undefined || release === item.release) {
    return null
  }
  if (release !== null && releases.some((entry) => entry.name === release && entry.closed)) {
    return null
  }
  return { release, optimistic: { release, effective_release: effectiveAfter(item, release, releases) } }
}
