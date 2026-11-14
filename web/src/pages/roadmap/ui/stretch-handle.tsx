import { useState, type PointerEvent } from 'react'
import type { Pricing } from '@/shared/api'
import { cn } from '@/shared/lib'
import { Tooltip } from '@/shared/ui'
import { snapSize, stretchedHours, type Size } from '../model/edits'

export interface Stretch {
  refusal: string | null
  size: string | null
  pricing: Pricing
  onResize: (size: Size) => void
}

const dragThreshold = 4

export function RefusedHandle({ at, label, refusal }: { at: number; label: string; refusal: string }) {
  return (
    <Tooltip content={refusal}>
      <button
        type="button"
        aria-disabled
        aria-label={`Resize ${label}: ${refusal}`}
        data-testid="refused-handle"
        className="absolute top-1/2 z-10 h-4 w-1.5 -translate-y-1/2 cursor-not-allowed rounded-sm opacity-0 group-hover/row:opacity-100 focus-visible:opacity-100 bg-muted-foreground/40"
        style={{ left: at - 3 }}
      />
    </Tooltip>
  )
}

export function StretchHandle({
  label,
  from,
  width,
  hours,
  stretch,
}: {
  label: string
  from: number
  width: number
  hours: number
  stretch: Stretch
}) {
  const [drag, setDrag] = useState<{ origin: number; delta: number } | null>(null)
  const snapped = drag === null ? null : snapSize(stretchedHours(hours, width, drag.delta), stretch.pricing)
  const ghost = snapped === null || hours <= 0 ? width : (width * stretch.pricing[snapped]) / hours

  const start = (event: PointerEvent<HTMLButtonElement>) => {
    event.preventDefault()
    event.stopPropagation()
    event.currentTarget.setPointerCapture?.(event.pointerId)
    setDrag({ origin: event.clientX, delta: 0 })
  }
  const move = (event: PointerEvent<HTMLButtonElement>) => {
    if (drag !== null) {
      setDrag({ origin: drag.origin, delta: event.clientX - drag.origin })
    }
  }
  const end = () => {
    if (drag !== null && snapped !== null && Math.abs(drag.delta) >= dragThreshold && snapped !== stretch.size) {
      stretch.onResize(snapped)
    }
    setDrag(null)
  }

  return (
    <>
      {drag !== null && snapped !== null && (
        <>
          <span
            data-testid="stretch-ghost"
            className="pointer-events-none absolute top-1/2 h-4 -translate-y-1/2 rounded-[3px] outline-2 -outline-offset-1 outline-dashed outline-primary bg-primary/10"
            style={{ left: from, width: Math.max(ghost, 3) }}
          />
          <span
            data-testid="stretch-preview"
            className="pointer-events-none absolute top-1/2 z-20 -translate-y-1/2 rounded bg-primary px-1.5 font-mono text-[10px] whitespace-nowrap text-primary-foreground"
            style={{ left: from + Math.max(ghost, 3) + 6 }}
          >
            {`${stretch.size ?? `~${stretch.pricing.default}`} → ${snapped} · ${stretch.pricing[snapped]} h`}
          </span>
        </>
      )}
      <button
        type="button"
        aria-label={`Resize ${label}`}
        data-testid="stretch-handle"
        onPointerDown={start}
        onPointerMove={move}
        onPointerUp={end}
        onPointerCancel={() => setDrag(null)}
        className={cn(
          'absolute top-1/2 z-10 h-4 w-2 -translate-y-1/2 cursor-ew-resize touch-none rounded-sm bg-foreground/50 opacity-0 group-hover/row:opacity-100 focus-visible:opacity-100',
          drag !== null && 'opacity-100',
        )}
        style={{ left: from + width - 4 }}
      />
    </>
  )
}
