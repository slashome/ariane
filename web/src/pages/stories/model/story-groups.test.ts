import { makeItem } from '@/shared/api'
import { groupStories } from './story-groups'

const story = (id: string, extra = {}) => makeItem({ id, kind: 'story', ...extra })

describe('groupStories', () => {
  const stories = [
    story('S3', { release: '1.0', milestone: '0.10' }),
    story('S1', { release: '1.0', milestone: '0.9', category: 'done', status: 'done' }),
    story('S2', { release: '1.0', milestone: '0.9', category: 'in_progress', status: 'in_progress' }),
    story('S4'),
  ]
  const tasks = [
    makeItem({ id: 'T1', parent: 'p#S2', category: 'done', status: 'done' }),
    makeItem({ id: 'T2', parent: 'p#S2' }),
    makeItem({ id: 'T3', parent: 'p#S3', category: 'dropped', status: 'dropped' }),
  ]

  it('groups by release and milestone in natural order, ungrouped last', () => {
    const groups = groupStories(stories, tasks, 'milestone')
    expect(groups.map((group) => group.label)).toEqual(['1.0 › 0.9', '1.0 › 0.10', 'No release or milestone'])
    expect(groups[0]?.items.map((row) => row.story.id)).toEqual(['S2', 'S1'])
  })

  it('measures each story and each group by the tasks whose parent it is', () => {
    const [first] = groupStories(stories, tasks, 'milestone')
    const s2 = first?.items.find((row) => row.story.id === 'S2')
    expect(s2?.progress.done).toBe(1)
    expect(s2?.progress.total).toBe(2)
    expect(first?.progress.ratio).toBe(0.5)
  })

  it('groups by epic, stories without one last', () => {
    const groups = groupStories([story('S5', { epic: 'p#E1' }), story('S6')], [], 'epic', (key) => `epic ${key}`)
    expect(groups.map((group) => group.label)).toEqual(['epic p#E1', 'No epic'])
  })

  it('groups by status in workflow order', () => {
    const groups = groupStories(stories, tasks, 'status')
    expect(groups.map((group) => group.key)).toEqual(['todo', 'in_progress', 'done'])
  })
})
