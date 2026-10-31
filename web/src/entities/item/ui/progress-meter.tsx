import type { Category } from '@/shared/api'
import { cn } from '@/shared/lib'
import type { Progress } from '../model/progress'
import { CategoryBar } from './category-bar'
import './item.css'

// The progress of a story or an epic: its children by category, and the share done as a number beside
// it; `empty` says what is missing when there are none.
export function ProgressMeter({
  progress,
  labels,
  legend = false,
  empty = 'no task',
  className,
}: {
  progress: Progress
  labels: (category: Category) => string
  legend?: boolean
  empty?: string
  className?: string
}) {
  return (
    <div className={cn('progress-meter', className)}>
      <CategoryBar counts={progress.counts} labels={labels} legend={legend} className="progress-meter__bar" />
      <span className="progress-meter__summary">
        {progress.total === 0 ? (
          empty
        ) : (
          <>
            <span className="progress-meter__ratio">{Math.round(progress.ratio * 100)}%</span> · {progress.done}/{progress.total}
          </>
        )}
      </span>
    </div>
  )
}
