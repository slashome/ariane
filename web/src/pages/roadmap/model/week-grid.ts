import type { BlockSpec, CalendarBlock } from '@/shared/api'

export const weekdays = ['mon', 'tue', 'wed', 'thu', 'fri', 'sat', 'sun'] as const

export type Weekday = (typeof weekdays)[number]

export interface Block {
  days: Weekday[]
  from: number
  to: number
}

export interface NodeBlocks {
  node: string
  blocks: Block[]
}

export interface GridPoint {
  day: number
  minute: number
}

export interface Overlap {
  block: number
  other: string | null
  otherBlock: number
}

export const snapMinutes = 30
export const dayMinutes = 24 * 60

const clamp = (value: number, low: number, high: number) => Math.min(Math.max(value, low), high)

export function snap(minute: number): number {
  return clamp(Math.round(minute / snapMinutes) * snapMinutes, 0, dayMinutes)
}

export function parseTime(text: string): number {
  const [hours, minutes] = text.split(':').map(Number)
  return (hours ?? 0) * 60 + (minutes ?? 0)
}

export function formatTime(minute: number): string {
  const hours = Math.floor(minute / 60)
  const minutes = minute % 60
  return `${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}`
}

function isWeekday(text: string): text is Weekday {
  return (weekdays as readonly string[]).includes(text)
}

export function sortDays(days: Iterable<Weekday>): Weekday[] {
  const set = new Set(days)
  return weekdays.filter((day) => set.has(day))
}

export function parseDays(text: string): Weekday[] {
  const days: Weekday[] = []
  for (const part of text.split(',').map((entry) => entry.trim())) {
    const [first, last] = part.split('-').map((entry) => entry.trim())
    if (first === undefined || !isWeekday(first)) {
      continue
    }
    const end = last !== undefined && isWeekday(last) ? last : first
    days.push(...weekdays.slice(weekdays.indexOf(first), weekdays.indexOf(end) + 1))
  }
  return sortDays(days)
}

export function formatDays(days: Weekday[]): string {
  const indexes = sortDays(days).map((day) => weekdays.indexOf(day))
  const runs: number[][] = []
  for (const index of indexes) {
    const run = runs[runs.length - 1]
    if (run !== undefined && run[run.length - 1] === index - 1) {
      run.push(index)
    } else {
      runs.push([index])
    }
  }
  return runs
    .flatMap((run) => (run.length >= 3 ? [`${weekdays[run[0] ?? 0]}-${weekdays[run[run.length - 1] ?? 0]}`] : run.map((index) => weekdays[index])))
    .join(',')
}

export function describeBlock(block: Block): string {
  return `${formatDays(block.days)} ${formatTime(block.from)}–${formatTime(block.to)}`
}

export function fromCalendarBlock(block: CalendarBlock): Block {
  const days = block.weekdays.filter(isWeekday)
  return { days: days.length > 0 ? sortDays(days) : parseDays(block.days), from: parseTime(block.from), to: parseTime(block.to) }
}

export function toBlockSpec(block: Block): BlockSpec {
  return { days: formatDays(block.days), from: formatTime(block.from), to: formatTime(block.to) }
}

export function drawnBlock(start: GridPoint, end: GridPoint): Block {
  const firstDay = clamp(Math.min(start.day, end.day), 0, weekdays.length - 1)
  const lastDay = clamp(Math.max(start.day, end.day), 0, weekdays.length - 1)
  let from = snap(Math.min(start.minute, end.minute))
  let to = snap(Math.max(start.minute, end.minute))
  if (to - from < snapMinutes) {
    from = Math.min(from, dayMinutes - snapMinutes)
    to = from + snapMinutes
  }
  return { days: weekdays.slice(firstDay, lastDay + 1), from, to }
}

export function movedBlock(block: Block, deltaDays: number, deltaMinutes: number): Block {
  const indexes = block.days.map((day) => weekdays.indexOf(day))
  const shift = clamp(deltaDays, -Math.min(...indexes), weekdays.length - 1 - Math.max(...indexes))
  const length = block.to - block.from
  const from = clamp(Math.round((block.from + deltaMinutes) / snapMinutes) * snapMinutes, 0, dayMinutes - length)
  return {
    days: indexes.map((index) => weekdays[index + shift] ?? 'mon'),
    from,
    to: from + length,
  }
}

export function resizedBlock(block: Block, edge: 'from' | 'to', minute: number): Block {
  const at = snap(minute)
  if (edge === 'from') {
    return { ...block, from: Math.min(at, block.to - snapMinutes) }
  }
  return { ...block, to: Math.max(at, block.from + snapMinutes) }
}

export function toggledDay(block: Block, day: Weekday): Block | null {
  const days = block.days.includes(day) ? block.days.filter((entry) => entry !== day) : sortDays([...block.days, day])
  return days.length === 0 ? null : { ...block, days }
}

export function replaced(blocks: Block[], index: number, block: Block | null): Block[] {
  return block === null ? blocks.filter((_, at) => at !== index) : blocks.map((entry, at) => (at === index ? block : entry))
}

export function shareTime(first: Block, second: Block): boolean {
  return first.days.some((day) => second.days.includes(day)) && first.from < second.to && second.from < first.to
}

export function overlapsOf(own: Block[], others: NodeBlocks[]): Overlap[] {
  const found: Overlap[] = []
  own.forEach((block, index) => {
    own.forEach((other, otherIndex) => {
      if (otherIndex > index && shareTime(block, other)) {
        found.push({ block: index, other: null, otherBlock: otherIndex })
      }
    })
    for (const { node, blocks } of others) {
      blocks.forEach((other, otherIndex) => {
        if (shareTime(block, other)) {
          found.push({ block: index, other: node, otherBlock: otherIndex })
        }
      })
    }
  })
  return found
}

export function weeklyHours(blocks: Block[]): number {
  return blocks.reduce((sum, block) => sum + (block.days.length * (block.to - block.from)) / 60, 0)
}

export function formatHours(hours: number): string {
  return String(Math.round(hours * 100) / 100)
}

export function focusSummary(hours: number, focus: number): string {
  return `${formatHours(hours)} h of blocks × ${formatHours(focus)} = ${formatHours(hours * focus)} h of focused work per week`
}
