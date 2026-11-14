import type { Calendar, CalendarChange, CalendarProblem, DaysOff, GlobalCalendarChange, NodeCalendarChange } from '@/shared/api'
import { fromCalendarBlock, toBlockSpec, type Block, type NodeBlocks } from './week-grid'

export interface NodeDraft {
  blocks: Block[]
  focus: number
  daysOff: string[]
}

export interface CalendarDraft {
  timezone: string
  daysOff: string[]
  node: NodeDraft | null
}

export type CalendarRequest =
  | { kind: 'remove'; node: string }
  | { kind: 'change'; change: CalendarChange }

export interface Refused {
  own: number[]
  others: { node: string; block: number }[]
}

export const defaultFocus = 1
export const minFocus = 0.05

export function formatDayOff(dayOff: DaysOff): string {
  return dayOff.from === dayOff.to ? dayOff.from : `${dayOff.from}..${dayOff.to}`
}

export function dayOffEntry(from: string, to: string): string | null {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(from)) {
    return null
  }
  if (to === '' || to === from) {
    return from
  }
  return /^\d{4}-\d{2}-\d{2}$/.test(to) && to > from ? `${from}..${to}` : null
}

export function withDayOff(list: string[], entry: string): string[] {
  return [...new Set([...list, entry])].sort()
}

export function savedDraft(calendar: Calendar, node: string | null): CalendarDraft {
  const table = node === null ? undefined : calendar.nodes[node]
  return {
    timezone: calendar.timezone,
    daysOff: calendar.days_off.map(formatDayOff),
    node:
      table === undefined
        ? null
        : { blocks: table.blocks.map(fromCalendarBlock), focus: table.focus, daysOff: table.days_off.map(formatDayOff) },
  }
}

export function ancestorsOf(node: string): string[] {
  if (node === '') {
    return []
  }
  const parts = node.split('/')
  return [...parts.slice(0, -1).map((_, index) => parts.slice(0, parts.length - 1 - index).join('/')), '']
}

export function carrierOf(calendar: Calendar, node: string): string | null {
  return [node, ...ancestorsOf(node)].find((candidate) => (calendar.nodes[candidate]?.blocks.length ?? 0) > 0) ?? null
}

export function otherBlocks(calendar: Calendar, node: string | null): NodeBlocks[] {
  return Object.entries(calendar.nodes)
    .filter(([path, table]) => path !== node && table.blocks.length > 0)
    .map(([path, table]) => ({ node: path, blocks: table.blocks.map(fromCalendarBlock) }))
}

function isEmptyNode(draft: NodeDraft | null): boolean {
  return draft === null || (draft.blocks.length === 0 && draft.focus === defaultFocus && draft.daysOff.length === 0)
}

function sameList<T>(first: T[], second: T[], same: (a: T, b: T) => boolean = (a, b) => a === b): boolean {
  return first.length === second.length && first.every((entry, index) => same(entry, second[index] as T))
}

export function sameBlock(first: Block, second: Block): boolean {
  return first.from === second.from && first.to === second.to && sameList(first.days, second.days)
}

function sameNode(first: NodeDraft | null, second: NodeDraft | null): boolean {
  if (isEmptyNode(first) || isEmptyNode(second)) {
    return isEmptyNode(first) && isEmptyNode(second)
  }
  return (
    first !== null &&
    second !== null &&
    first.focus === second.focus &&
    sameList(first.daysOff, second.daysOff) &&
    sameList(first.blocks, second.blocks, sameBlock)
  )
}

export function sameDraft(first: CalendarDraft, second: CalendarDraft): boolean {
  return first.timezone === second.timezone && sameList(first.daysOff, second.daysOff) && sameNode(first.node, second.node)
}

export function requestsFor(saved: CalendarDraft, draft: CalendarDraft, node: string | null): CalendarRequest[] {
  const requests: CalendarRequest[] = []
  if (node !== null && !sameNode(saved.node, draft.node)) {
    if (draft.node === null || draft.node.blocks.length === 0) {
      if (saved.node !== null) {
        requests.push({ kind: 'remove', node })
      }
    } else {
      const before = saved.node
      const change: NodeCalendarChange = { node }
      if (before === null || !sameList(before.blocks, draft.node.blocks, sameBlock)) {
        change.blocks = draft.node.blocks.map(toBlockSpec)
      }
      if ((before?.focus ?? defaultFocus) !== draft.node.focus) {
        change.focus = draft.node.focus === defaultFocus ? null : draft.node.focus
      }
      if (!sameList(before?.daysOff ?? [], draft.node.daysOff)) {
        change.days_off = draft.node.daysOff
      }
      requests.push({ kind: 'change', change })
    }
  }
  const globals: GlobalCalendarChange = {}
  if (saved.timezone !== draft.timezone) {
    globals.timezone = draft.timezone
  }
  if (!sameList(saved.daysOff, draft.daysOff)) {
    globals.days_off = draft.daysOff
  }
  if (Object.keys(globals).length > 0) {
    requests.push({ kind: 'change', change: globals })
  }
  return requests
}

export function refusedBlocks(problems: CalendarProblem[], node: string): Refused {
  const refused: Refused = { own: [], others: [] }
  for (const problem of problems) {
    if (problem.kind !== 'overlap' || problem.block === undefined || problem.other_block === undefined) {
      continue
    }
    const pairs: [string | null | undefined, number][] = [
      [problem.node, problem.block],
      [problem.other, problem.other_block],
    ]
    for (const [owner, block] of pairs) {
      if (owner === node) {
        refused.own.push(block)
      } else if (owner !== null && owner !== undefined) {
        refused.others.push({ node: owner, block })
      }
    }
  }
  return refused
}

export function isWarning(problem: CalendarProblem): boolean {
  return problem.kind === 'unknown_node' || problem.kind === 'unused'
}
