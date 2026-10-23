import type { Item, Person } from '@/shared/api'

// The names of the people an item waits for, from its node's directory; an unknown handle shows as is.
export function waitingNames(item: Pick<Item, 'waiting_for'>, people: Record<string, Person>): string[] {
  return item.waiting_for.map((handle) => people[handle]?.name ?? handle)
}

// Whole days between two `YYYY-MM-DD` dates; `null` when `since` is missing or unreadable.
export function daysBetween(since: string | null, today: string): number | null {
  if (since === null) {
    return null
  }
  const start = Date.parse(`${since}T00:00:00Z`)
  const end = Date.parse(`${today}T00:00:00Z`)
  return Number.isNaN(start) || Number.isNaN(end) ? null : Math.max(0, Math.round((end - start) / 86_400_000))
}

// Today as `YYYY-MM-DD`, in the browser's time zone.
export function today(now: Date = new Date()): string {
  const pad = (value: number) => String(value).padStart(2, '0')
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`
}

// `Keyvan, Doryan · 3 d` — who, and for how long.
export function waitingSummary(item: Pick<Item, 'waiting_for' | 'waiting_since'>, people: Record<string, Person>, on: string = today()): string {
  const names = waitingNames(item, people).join(', ')
  const days = daysBetween(item.waiting_since, on)
  return days === null ? names : `${names} · ${days} d`
}
