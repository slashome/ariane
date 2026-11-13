import { useCallback, useMemo, useState } from 'react'
import { defaultZoom, makeScale, zooms, type Scale } from './scale'

export const todayAt = 0.2

const step = 0.75

const fallbackWidth = 960

export interface GanttView {
  scale: Scale
  left: number
  width: number
  dated: boolean
  canZoomIn: boolean
  canZoomOut: boolean
  today: () => void
  earlier: () => void
  later: () => void
  zoomIn: () => void
  zoomOut: () => void
  reveal: (left: number) => void
}

export function useElementWidth(): [number, (element: HTMLElement | null) => (() => void) | undefined] {
  const [width, setWidth] = useState(fallbackWidth)
  const measure = useCallback((element: HTMLElement | null) => {
    if (element === null || typeof ResizeObserver === 'undefined') {
      return undefined
    }
    const observer = new ResizeObserver(([entry]) => {
      if (entry && entry.contentRect.width > 0) {
        setWidth(Math.round(entry.contentRect.width))
      }
    })
    observer.observe(element)
    return () => observer.disconnect()
  }, [])
  return [width, measure]
}

function zoomAt(index: number): number {
  return zooms[Math.min(Math.max(index, 0), zooms.length - 1)] ?? 1
}

export function useGanttView(dated: boolean, width: number): GanttView {
  const [zoom, setZoom] = useState<number | null>(null)
  const [offset, setOffset] = useState<number | null>(null)
  const index = zoom ?? defaultZoom(dated)
  const scale = useMemo(() => makeScale(zoomAt(index), dated), [index, dated])
  const left = offset ?? -Math.round(width * todayAt)

  const zoomTo = (next: number) => {
    if (next < 0 || next >= zooms.length) {
      return
    }
    if (offset !== null) {
      const centre = scale.time(left + width / 2)
      setOffset(Math.round(makeScale(zoomAt(next), dated).x(centre) - width / 2))
    }
    setZoom(next)
  }

  return {
    scale,
    left,
    width,
    dated,
    canZoomIn: index < zooms.length - 1,
    canZoomOut: index > 0,
    today: () => setOffset(null),
    earlier: () => setOffset(Math.round(left - width * step)),
    later: () => setOffset(Math.round(left + width * step)),
    zoomIn: () => zoomTo(index + 1),
    zoomOut: () => zoomTo(index - 1),
    reveal: (next) => setOffset(Math.round(next)),
  }
}
