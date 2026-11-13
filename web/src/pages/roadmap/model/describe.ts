import { formatHours } from '@/entities/item'
import type { Moment, WorkStage } from '@/shared/api'
import type { Fragment, GanttRow, WorkBar } from './rows'

const stageLabels: Record<WorkStage, string> = {
  open: 'planned',
  in_progress: 'in progress',
  testing: 'in test',
  waiting: 'waiting',
  done: 'done',
}

export function formatMoment(moment: Moment | null, dated: boolean): string | null {
  if (moment === null) {
    return null
  }
  if (dated && moment.at !== null) {
    return moment.at.slice(0, 16).replace('T', ' ')
  }
  return `+${Number(moment.hours.toFixed(1))} h`
}

export function hoursLabel(hours: number, testHours: number, guessed: boolean): string {
  const total = hours + testHours
  return total > 0 ? `${guessed ? '~' : ''}${formatHours(total)}` : ''
}

function targetLine(row: GanttRow): string | null {
  if (row.target === null && row.missed === null) {
    return null
  }
  const target = row.target === null ? 'target' : `target ${row.target}`
  return row.missed === null ? target : `${target} — missed: ${row.missed}`
}

function heading(row: GanttRow): string {
  return row.level === 'release' ? `Release ${row.title}` : [row.id, row.title].filter(Boolean).join(' ')
}

function basisLine(work: WorkBar): string | null {
  switch (work.basis) {
    case 'default':
      return 'no size: counted at the default size'
    case 'provisional':
      return 'provisional size, until it is split'
    default:
      return null
  }
}

export function describeWork(row: GanttRow, work: WorkBar, dated: boolean): string[] {
  const lines = [heading(row), stageLabels[work.stage]]
  if (work.stage === 'done') {
    lines.push(
      work.closed === null
        ? 'closed, date unknown'
        : work.closed.approximate
          ? `closed ~${work.closed.date} (approximate, from git)`
          : `closed ${work.closed.date}`,
    )
  } else {
    const tester = work.tester === 'owner' ? 'by the owner' : 'by others, as a delay'
    lines.push(
      `${work.hatched ? '~' : ''}${formatHours(work.hours)} of work + ${formatHours(work.testHours)} of testing ${tester}`,
    )
    const basis = basisLine(work)
    if (basis) {
      lines.push(basis)
    }
    const start = formatMoment(work.start, dated)
    const delivered = formatMoment(work.delivered, dated)
    if (start && delivered) {
      lines.push(`${start} → ${delivered}`)
    }
  }
  const target = targetLine(row)
  return target ? [...lines, target] : lines
}

export function describeFragment(row: GanttRow, fragment: Fragment, dated: boolean, now: string): string[] {
  const lines = [heading(row)]
  if (row.level !== 'release') {
    lines.push(`release ${fragment.release}`)
  }
  lines.push(`${row.done} done, ${row.open} open`)
  if (row.hours + row.testHours > 0) {
    lines.push(`${formatHours(row.hours)} of work + ${formatHours(row.testHours)} of testing left`)
  }
  lines.push(`${spanEnd(fragment.start, dated, now)} → ${spanEnd(fragment.end, dated, now)}`)
  const target = targetLine(row)
  return target ? [...lines, target] : lines
}

function spanEnd(time: number, dated: boolean, now: string): string {
  if (time < 0 || dated) {
    const date = new Date(Date.parse(now) + time * 3_600_000)
    const pad = (value: number) => String(value).padStart(2, '0')
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
  }
  return `+${Number(time.toFixed(1))} h`
}
