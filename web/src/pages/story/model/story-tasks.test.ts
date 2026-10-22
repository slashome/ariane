import { makeItem } from '@/shared/api'
import { storyTasks } from './story-tasks'

describe('storyTasks', () => {
  it('orders the tasks by dependency and tells which can start', () => {
    const design = makeItem({ id: 'T1', parent: 'p#S1', category: 'done', status: 'done' })
    const build = makeItem({ id: 'T2', parent: 'p#S1', depends_on: ['p#T3'] })
    const prepare = makeItem({ id: 'T3', parent: 'p#S1', depends_on: ['p#T1', 'q#T9'] })
    const outside = makeItem({ node: 'q', id: 'T9', category: 'done', status: 'done' })
    const rows = storyTasks([build, design, prepare], [build, design, prepare, outside])
    expect(rows.map((row) => row.task.id)).toEqual(['T1', 'T3', 'T2'])
    expect(rows.map((row) => row.ready)).toEqual([false, true, false])
    expect(rows[2]?.blockers.open.map((item) => item.id)).toEqual(['T3'])
  })
})
