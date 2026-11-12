import { makeItem } from '@/shared/api'
import { groupStories } from './story-groups'

const story = (id: string, extra = {}) => makeItem({ id, kind: 'story', ...extra })
const planned = (name: string, position: number | null = null, source: 'own' | 'epic' = 'own') => ({
  release: source === 'own' ? name : null,
  effective_release: { name, source, from: null, position, parking: false },
})

describe('groupStories', () => {
  const stories = [
    story('S3', planned('0.10')),
    story('S1', { ...planned('0.9'), category: 'done', status: 'done' }),
    story('S2', { ...planned('0.9'), category: 'in_progress', status: 'in_progress' }),
    story('S4'),
  ]
  const tasks = [
    makeItem({ id: 'T1', parent: 'p#S2', category: 'done', status: 'done' }),
    makeItem({ id: 'T2', parent: 'p#S2' }),
    makeItem({ id: 'T3', parent: 'p#S3', category: 'dropped', status: 'dropped' }),
  ]

  it('groups by release in natural order, ungrouped last', () => {
    const groups = groupStories(stories, tasks, 'release')
    expect(groups.map((group) => group.label)).toEqual(['0.9', '0.10', 'No release'])
    expect(groups[0]?.items.map((row) => row.story.id)).toEqual(['S2', 'S1'])
  })

  it('groups a story by the release its epic gives it when it carries none', () => {
    const groups = groupStories([story('S5', planned('1.0', null, 'epic')), story('S6', { release: '2.0' })], [], 'release')
    expect(groups.map((group) => group.label)).toEqual(['1.0', 'No release'])
  })

  it('orders the release groups by the release order the node declares, undeclared ones after', () => {
    const declared = [story('S5', planned('plus-tard', 2)), story('S6', planned('0.10', 0)), story('S7', planned('0.9', 1)), story('S8', planned('9.9'))]
    const groups = groupStories(declared, [], 'release')
    expect(groups.map((group) => group.label)).toEqual(['0.10', '0.9', 'plus-tard', '9.9'])
  })

  it('measures each story and each group by the tasks whose parent it is', () => {
    const [first] = groupStories(stories, tasks, 'release')
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

  it('orders the stories of a group, and their tasks, by rank before id', () => {
    const ranked = [story('S8'), story('S9', { rank: 1000 })]
    const children = [makeItem({ id: 'T8', parent: 'p#S9' }), makeItem({ id: 'T9', parent: 'p#S9', rank: 1000 })]
    const [group] = groupStories(ranked, children, 'release')
    expect(group?.items.map((row) => row.story.id)).toEqual(['S9', 'S8'])
    expect(group?.items[0]?.tasks.map((task) => task.id)).toEqual(['T9', 'T8'])
  })
})
