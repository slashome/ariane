import { syncMessage } from './sync-message'

const base = { item: 'p#T1', ticket: 9, category: 'waiting' as const }

describe('syncMessage', () => {
  it('says what went to monday, and who was told', () => {
    expect(syncMessage({ applied: true, steps: [{ ...base, action: 'push', from: '0', to: '2', mention: [['Keyvan', 42]] }] })).toEqual({
      tone: 'success',
      text: 'monday:9 moved to label 2, Keyvan told',
    })
  })

  it('warns about a conflict or a refusal, and keeps quiet when in step', () => {
    expect(syncMessage({ applied: true, steps: [{ ...base, action: 'conflict', monday: '8', last: null }] })?.text).toContain('never synced yet')
    expect(syncMessage({ applied: true, steps: [{ ...base, action: 'refused', reason: 'no label' }] })?.tone).toBe('warning')
    expect(syncMessage({ applied: true, steps: [{ ...base, action: 'in_step' }] })).toBeNull()
    expect(syncMessage({ applied: true, steps: [] })).toBeNull()
  })
})
