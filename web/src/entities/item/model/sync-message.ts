import type { SyncReport } from '@/shared/api'

export interface SyncMessage {
  tone: 'success' | 'warning'
  text: string
}

// What to tell after sending one item to monday; `null` when there was nothing to send.
export function syncMessage(report: SyncReport): SyncMessage | null {
  const step = report.steps[0]
  if (step === undefined || step.action === 'in_step') {
    return null
  }
  switch (step.action) {
    case 'push': {
      const told = step.mention.length > 0 ? `, ${step.mention.map(([name]) => name).join(', ')} told` : ''
      return { tone: 'success', text: `monday:${step.ticket} moved to label ${step.to}${told}` }
    }
    case 'conflict':
      return {
        tone: 'warning',
        text: `monday:${step.ticket} not moved: ${step.last === null ? 'never synced yet' : 'it changed in monday too'} — run \`ariane sync monday\` to settle it`,
      }
    case 'refused':
      return { tone: 'warning', text: `monday:${step.ticket} not moved: ${step.reason}` }
  }
}
