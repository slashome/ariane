import type { Estimate } from '@/shared/api'

export function formatHours(hours: number): string {
  return `${Number(hours.toFixed(2))} h`
}

export function isGuessed(estimate: Estimate): boolean {
  return estimate.basis === 'default' || estimate.basis === 'provisional' || estimate.guessed > 0
}

export function estimateHours(estimate: Estimate): string {
  return `${isGuessed(estimate) ? '~' : ''}${formatHours(estimate.hours)}`
}

function partsOf(estimate: Estimate): string {
  const parts = `${estimate.parts} ${estimate.parts === 1 ? 'part' : 'parts'}`
  return estimate.guessed > 0 ? `${parts}, ${estimate.guessed} guessed from a default or provisional size` : parts
}

export function estimateBasis(estimate: Estimate): string {
  switch (estimate.basis) {
    case 'size':
      return `Size ${estimate.size}`
    case 'provisional':
      return `Provisional size ${estimate.size}, until it is split`
    case 'default':
      return `No size: counted as the default size ${estimate.size}`
    case 'tasks':
      return estimate.parts === 0 ? 'Every part is dropped: nothing left to do' : `Sum of ${partsOf(estimate)}`
  }
}

export function estimateTesting(estimate: Estimate): string | null {
  if (estimate.test_hours <= 0) {
    return null
  }
  const who = estimate.tester === 'owner' ? 'by the owner' : 'by others, as a delay'
  return `+${formatHours(estimate.test_hours)} of testing ${who}`
}

export function estimateSummary(estimate: Estimate): string {
  const lines = [`${formatHours(estimate.hours)} of focused work`, estimateBasis(estimate)]
  const testing = estimateTesting(estimate)
  if (testing) {
    lines.push(testing)
  }
  if (estimate.remaining_hours !== estimate.hours) {
    lines.push(`${formatHours(estimate.remaining_hours)} remaining`)
  }
  return lines.join('\n')
}
