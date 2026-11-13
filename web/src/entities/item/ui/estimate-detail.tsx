import type { Estimate } from '@/shared/api'
import { cn } from '@/shared/lib'
import { estimateBasis, estimateHours, estimateTesting, formatHours, isGuessed } from '../model/estimate'

export function EstimateDetail({ estimate, className }: { estimate: Estimate; className?: string }) {
  const guessed = isGuessed(estimate)
  const testing = estimateTesting(estimate)
  const remaining = estimate.remaining_hours !== estimate.hours ? `${formatHours(estimate.remaining_hours)} remaining` : null
  return (
    <div className={cn('space-y-0.5 text-xs text-muted-foreground', className)} aria-label="Estimate">
      <p>
        <span
          data-guessed={guessed || undefined}
          className={cn('font-mono font-medium text-foreground', guessed && 'underline decoration-muted-foreground/60 decoration-dashed underline-offset-2')}
        >
          {estimateHours(estimate)}
        </span>{' '}
        — {estimateBasis(estimate)}
      </p>
      {testing && <p>{testing}</p>}
      {remaining && <p>{remaining}</p>}
    </div>
  )
}
