import { cleanup, render, screen } from '@testing-library/react'
import { MemoryRouter } from 'react-router'
import { cardStats } from '../model/card-stats'
import { NodeCard } from './node-card'

const stats = cardStats([], [], 'cmc/lib')

afterEach(cleanup)

describe('NodeCard', () => {
  it('renders the node description as Markdown', async () => {
    render(
      <MemoryRouter>
        <NodeCard path="cmc/lib" description={'Design system **flow**.\n\n- CI aligned on cmc-lib'} stats={stats} labels={(category) => category} />
      </MemoryRouter>,
    )
    expect((await screen.findByText('flow')).tagName).toBe('STRONG')
    expect(screen.getByText('CI aligned on cmc-lib').tagName).toBe('LI')
  })

  it('shows no About section without a description', () => {
    render(
      <MemoryRouter>
        <NodeCard path="cmc/lib" description={null} stats={stats} labels={(category) => category} />
      </MemoryRouter>,
    )
    expect(screen.queryByText('About')).toBeNull()
  })
})
