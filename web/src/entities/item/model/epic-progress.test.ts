import { makeItem } from '@/shared/api'
import { epicProgress } from './epic-progress'

describe('epicProgress', () => {
  it('counts the stories of an epic and the tasks of those stories', () => {
    const epic = makeItem({ id: 'E1', kind: 'epic' })
    const items = [
      epic,
      makeItem({ id: 'S1', kind: 'story', parent: 'p#E1', category: 'done', status: 'done' }),
      makeItem({ id: 'S2', kind: 'story', parent: 'p#E1' }),
      makeItem({ id: 'S3', kind: 'story' }),
      makeItem({ id: 'T1', parent: 'p#S1', category: 'done', status: 'done' }),
      makeItem({ id: 'T2', parent: 'p#S2' }),
      makeItem({ id: 'T3', parent: 'p#S3' }),
      makeItem({ id: 'T4', parent: 'p#E1' }),
    ]
    const progress = epicProgress(epic, items)
    expect(progress.stories.map((story) => story.id)).toEqual(['S1', 'S2'])
    expect(progress.tasks.map((task) => task.id)).toEqual(['T1', 'T2'])
    expect(progress.storyProgress.ratio).toBe(0.5)
    expect(progress.taskProgress.done).toBe(1)
  })
})
