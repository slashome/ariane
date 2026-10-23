import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { makeItem } from '@/shared/api'
import { WaitingDialog } from './waiting-dialog'

describe('WaitingDialog', () => {
  it('asks for at least one person and a reason', async () => {
    const onConfirm = vi.fn()
    render(
      <WaitingDialog
        item={makeItem({ title: 'Ship it' })}
        people={{ keyvan: { name: 'Keyvan Coen' }, doryan: { name: 'Doryan' } }}
        onConfirm={onConfirm}
        onCancel={() => undefined}
      />,
    )
    const wait = screen.getByRole('button', { name: 'Wait' })
    expect(wait).toBeDisabled()
    await userEvent.click(screen.getByRole('checkbox', { name: /Keyvan Coen/ }))
    await userEvent.click(screen.getByRole('radio', { name: 'A review' }))
    await userEvent.click(wait)
    expect(onConfirm).toHaveBeenCalledWith({ waiting_for: ['keyvan'], waiting_on: 'review' })
  })

  it('says where to declare people when the node has none', () => {
    render(<WaitingDialog item={makeItem()} people={{}} onConfirm={() => undefined} onCancel={() => undefined} />)
    expect(screen.getByText(/declares no one to wait for/)).toBeInTheDocument()
  })
})
