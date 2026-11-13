import type { GanttRow } from './rows'
import type { Scale } from './scale'

export type Edge = 'before' | 'after'

export interface Extent {
  start: number
  end: number
}

export interface Viewport {
  scale: Scale
  left: number
  width: number
}

const margin = 0.1

export function rowExtent(row: Pick<GanttRow, 'work' | 'fragments'>): Extent | null {
  const spans = row.work ? row.work.segments : row.fragments
  if (spans.length === 0) {
    return null
  }
  return {
    start: Math.min(...spans.map((span) => span.start)),
    end: Math.max(...spans.map((span) => span.end)),
  }
}

export function edgeOf(extent: Extent | null, { scale, left, width }: Viewport): Edge | null {
  if (extent === null) {
    return null
  }
  if (scale.x(extent.end) - left < 0) {
    return 'before'
  }
  return scale.x(extent.start) - left > width ? 'after' : null
}

export function revealLeft(extent: Extent, edge: Edge, { scale, width }: Pick<Viewport, 'scale' | 'width'>): number {
  const gap = width * margin
  const from = scale.x(extent.start)
  const to = scale.x(extent.end)
  if (edge === 'before' || to - from > width - 2 * gap) {
    return Math.round(from - gap)
  }
  return Math.round(to - width + gap)
}
