import type { Category } from '@/shared/api'
import { cn } from '@/shared/lib'
import type { Progress } from '../model/progress'
import { CategoryBar } from './category-bar'

// The progress of a story: its children by category, and the share done as a number beside it.
export function ProgressMeter({
  progress,
  labels,
  legend = false,
  className,
}: {
  progress: Progress
  labels: (category: Category) => string
  legend?: boolean
  className?: string
}) {
  return (
    <div className={cn('flex items-start gap-3', className)}>
      <CategoryBar counts={progress.counts} labels={labels} legend={legend} className="min-w-0 flex-1 pt-1" />
      <span className="w-20 shrink-0 text-right text-xs tabular-nums text-muted-foreground">
        {progress.total === 0 ? (
          'no task'
        ) : (
          <>
            <span className="font-medium text-foreground">{Math.round(progress.ratio * 100)}%</span> · {progress.done}/{progress.total}
          </>
        )}
      </span>
    </div>
  )
}
