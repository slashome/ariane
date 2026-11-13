export const zooms: readonly number[] = [0.25, 0.5, 1, 2, 4, 8, 16, 32]

export function defaultZoom(dated: boolean): number {
  return zooms.indexOf(dated ? 1 : 2)
}

export const pastHoursPerDay = 8

export interface Scale {
  pxPerHour: number
  pastFactor: number
  x: (time: number) => number
  time: (x: number) => number
}

export function makeScale(pxPerHour: number, dated: boolean): Scale {
  const pastFactor = dated ? 1 : pastHoursPerDay / 24
  return {
    pxPerHour,
    pastFactor,
    x: (time) => (time >= 0 ? time * pxPerHour : time * pxPerHour * pastFactor),
    time: (x) => (x >= 0 ? x / pxPerHour : x / (pxPerHour * pastFactor)),
  }
}

export interface Tick {
  x: number
  label: string
  major: boolean
}

const hour = 3_600_000

const dayFormat = new Intl.DateTimeFormat('en-GB', { day: 'numeric', month: 'short' })
const monthFormat = new Intl.DateTimeFormat('en-GB', { month: 'short', year: 'numeric' })

type DateStep = 'day' | 'week' | 'month'

function dateStep(pxPerDay: number): DateStep {
  if (pxPerDay >= 36) {
    return 'day'
  }
  return pxPerDay * 7 >= 48 ? 'week' : 'month'
}

function startOfDay(ms: number): Date {
  const date = new Date(ms)
  return new Date(date.getFullYear(), date.getMonth(), date.getDate())
}

function dateTicks(scale: Scale, now: number, from: number, to: number): Tick[] {
  const step = dateStep(scale.pxPerHour * scale.pastFactor * 24)
  const first = startOfDay(now + scale.time(from) * hour)
  const last = now + scale.time(to) * hour
  const ticks: Tick[] = []
  for (let day = first; day.getTime() <= last; day = new Date(day.getFullYear(), day.getMonth(), day.getDate() + 1)) {
    const firstOfMonth = day.getDate() === 1
    const monday = day.getDay() === 1
    const x = scale.x((day.getTime() - now) / hour)
    if ((step === 'week' && !monday) || (step === 'month' && !firstOfMonth) || x < from) {
      continue
    }
    ticks.push({
      x,
      label: step === 'month' ? monthFormat.format(day) : dayFormat.format(day),
      major: step === 'month' || firstOfMonth || (step === 'day' && monday),
    })
  }
  return ticks
}

const hourSteps: readonly number[] = [1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 2000, 5000]

function hourTicks(scale: Scale, from: number, to: number): Tick[] {
  const step = hourSteps.find((entry) => entry * scale.pxPerHour >= 64) ?? 10_000
  const ticks: Tick[] = []
  for (let hours = Math.max(1, Math.ceil(scale.time(from) / step)) * step; scale.x(hours) <= to; hours += step) {
    ticks.push({ x: scale.x(hours), label: `+${hours} h`, major: hours % (step * 5) === 0 })
  }
  return ticks
}

export function ticks(scale: Scale, dated: boolean, now: number, from: number, to: number): Tick[] {
  if (dated) {
    return dateTicks(scale, now, from, to)
  }
  const past = from < 0 ? dateTicks(scale, now, from, Math.min(to, 0)).filter((tick) => tick.x < -24) : []
  return [...past, ...(to >= 0 ? hourTicks(scale, from, to) : [])]
}
