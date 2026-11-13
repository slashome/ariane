import { ChevronLeft, ChevronRight, ZoomIn, ZoomOut } from 'lucide-react'
import { Button } from '@/shared/ui'
import type { GanttView } from '../model/view'

export function GanttNav({ view }: { view: GanttView }) {
  return (
    <div role="group" aria-label="Time" className="flex items-center gap-1">
      <Button variant="ghost" size="icon" aria-label="Earlier" title="Earlier" onClick={view.earlier}>
        <ChevronLeft className="size-4" aria-hidden />
      </Button>
      <Button variant="outline" size="sm" onClick={view.today}>
        Today
      </Button>
      <Button variant="ghost" size="icon" aria-label="Later" title="Later" onClick={view.later}>
        <ChevronRight className="size-4" aria-hidden />
      </Button>
      <span className="mx-1 h-5 w-px bg-border" aria-hidden />
      <Button variant="ghost" size="icon" aria-label="Zoom out" title="Zoom out" disabled={!view.canZoomOut} onClick={view.zoomOut}>
        <ZoomOut className="size-4" aria-hidden />
      </Button>
      <Button variant="ghost" size="icon" aria-label="Zoom in" title="Zoom in" disabled={!view.canZoomIn} onClick={view.zoomIn}>
        <ZoomIn className="size-4" aria-hidden />
      </Button>
    </div>
  )
}
