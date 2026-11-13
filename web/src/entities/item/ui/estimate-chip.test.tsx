import { render, screen } from '@testing-library/react'
import { makeItem, type Estimate } from '@/shared/api'
import { EstimateChip } from './estimate-chip'
import { ItemCard } from './item-card'

const estimate: Estimate = {
  hours: 12,
  remaining_hours: 12,
  test_hours: 3,
  remaining_test_hours: 3,
  tester: 'owner',
  basis: 'size',
  size: 'M',
  parts: 1,
  guessed: 0,
}

describe('EstimateChip', () => {
  it('shows the hours next to a set size, plainly', () => {
    render(<EstimateChip item={makeItem({ size: 'M', estimate })} />)
    const chip = screen.getByText('size M · 12 h')
    expect(chip).not.toHaveAttribute('data-guessed')
    expect(chip.title).toContain('+3 h of testing by the owner')
  })

  it('marks an assumed default size as a guess', () => {
    render(<EstimateChip item={makeItem({ estimate: { ...estimate, basis: 'default' } })} />)
    expect(screen.getByText('~12 h')).toHaveAttribute('data-guessed', 'true')
  })

  it('shows the sum of a split story instead of its own size', () => {
    const story = makeItem({ kind: 'story', size: 'XS', estimate: { ...estimate, basis: 'tasks', size: null, hours: 36, parts: 3 } })
    render(<ItemCard item={story} />)
    expect(screen.getByText('36 h')).not.toHaveAttribute('data-guessed')
    expect(screen.queryByText(/size XS/)).toBeNull()
  })

  it('falls back on the bare size when the item carries no estimate', () => {
    render(<EstimateChip item={makeItem({ kind: 'reflexion', size: 'S' })} />)
    expect(screen.getByText('size S')).toBeInTheDocument()
  })
})
