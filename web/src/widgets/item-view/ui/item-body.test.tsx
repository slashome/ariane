import { render, screen } from '@testing-library/react'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { makeItem } from '@/shared/api'
import { ItemBody } from './item-body'

describe('ItemBody', () => {
  it('turns links to other items into routes and keeps external links outside', () => {
    const task = makeItem({ id: 'T1', path: 'tree/p/tasks/T1-a.md' })
    const story = makeItem({ id: 'S1', kind: 'story', path: 'tree/p/stories/S1-s.md' })
    const body = 'Part of [the story](../stories/S1-s.md), see [docs](https://example.com) and [gone](./T9.md).\n\n| a | b |\n|---|---|\n| 1 | 2 |\n'
    const router = createMemoryRouter([{ path: '*', element: <ItemBody item={task} body={body} items={[task, story]} /> }], {
      initialEntries: ['/tasks/p/T1?node=p&q=x'],
    })
    render(<RouterProvider router={router} />)
    expect(screen.getByRole('link', { name: 'the story' })).toHaveAttribute('href', '/stories/p/S1?node=p')
    expect(screen.getByRole('link', { name: 'docs' })).toHaveAttribute('target', '_blank')
    expect(screen.queryByRole('link', { name: 'gone' })).not.toBeInTheDocument()
    expect(screen.getByRole('table')).toBeInTheDocument()
  })
})
