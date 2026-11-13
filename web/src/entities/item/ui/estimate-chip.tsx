import type { Item } from '@/shared/api'
import { cn } from '@/shared/lib'
import { estimateHours, estimateSummary, isGuessed } from '../model/estimate'

const chip = 'rounded px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground'

export function EstimateChip({ item, className }: { item: Item; className?: string }) {
  const { estimate } = item
  if (!estimate) {
    return item.size ? <span className={cn(chip, 'bg-muted', className)}>size {item.size}</span> : null
  }
  const guessed = isGuessed(estimate)
  const sized = estimate.basis === 'size' || estimate.basis === 'provisional'
  return (
    <span
      title={estimateSummary(estimate)}
      data-guessed={guessed || undefined}
      className={cn(chip, guessed ? 'border border-dashed border-muted-foreground/40' : 'bg-muted', className)}
    >
      {sized && `size ${estimate.size} · `}
      {estimateHours(estimate)}
    </span>
  )
}
