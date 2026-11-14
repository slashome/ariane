import { makeItem, type Item, type Pricing } from '@/shared/api'
import { checkDrop, currentSize, groupOf, hasGrip, siblingMove, snapSize, stepSize, stretchedHours, stretchRefusal } from './edits'
import { makeRoadmap, makeEpic, makeRelease, makeStory, makeWork, planned, sampleRoadmap } from './fixtures'
import { buildRows, type GanttRow } from './rows'

const pricing: Pricing = { XS: 2, S: 4, M: 12, L: 24, XL: 48, default: 'M', test_ratio: 0.25, tester: 'owner' }

const items = new Map<string, Item>(
  ['T1', 'T2', 'T3', 'T4', 'S1', 'S2', 'S3', 'E1', 'E2'].map((id) => [
    `p#${id}`,
    makeItem({ id, kind: id.startsWith('T') ? 'task' : id.startsWith('S') ? 'story' : 'epic', path: `tree/p/${id}.md` }),
  ]),
)
const itemOf = (row: GanttRow) => (row.reference === null ? undefined : items.get(row.reference))

const rows = buildRows(sampleRoadmap(), ['release', 'epic', 'story', 'task'])
const row = (reference: string, release = '0.1') => {
  const found = rows.find((entry) => entry.reference === reference && entry.key.includes(`R:p:${release}`))
  if (found === undefined) {
    throw new Error(`no row ${reference}`)
  }
  return found
}

describe('reordering Gantt rows', () => {
  it('groups a row with the rows drawn under the same parent', () => {
    expect(groupOf(row('p#T1'))).toBe(groupOf(row('p#T2')))
    expect(groupOf(row('p#S1'))).toBe(groupOf(row('p#S2')))
    expect(groupOf(row('p#T2'))).not.toBe(groupOf(row('p#T4', '0.2')))
  })

  it('accepts a drop next to a sibling of the same level and group', () => {
    expect(checkDrop(row('p#T2'), row('p#T1'), 'before', itemOf)).toEqual({ ok: true, other: row('p#T1'), place: 'before' })
    expect(checkDrop(row('p#S2'), row('p#S1'), 'after', itemOf)).toMatchObject({ ok: true, place: 'after' })
  })

  it('refuses a drop in another group, at another level, or on a row with no item', () => {
    expect(checkDrop(row('p#T2'), row('p#T4', '0.2'), 'before', itemOf)).toMatchObject({ ok: false, reason: expect.stringContaining('among its siblings') })
    expect(checkDrop(row('p#T2'), row('p#S2'), 'before', itemOf)).toMatchObject({ ok: false })
    expect(checkDrop(row('p#T2'), row('p#T2'), 'before', itemOf)).toMatchObject({ ok: false })
    const unnamed = rows.find((entry) => entry.level === 'epic' && entry.reference === null) as GanttRow
    expect(checkDrop(row('p#E1'), unnamed, 'after', itemOf)).toMatchObject({ ok: false, reason: 'This row has no item to order' })
  })

  it('refuses a drop next to an item of another node, whose order is separate', () => {
    const foreign = new Map(items)
    foreign.set('p#T1', makeItem({ id: 'T1', node: 'p/child', path: 'tree/p/child/T1.md' }))
    const reference = (entry: GanttRow) => (entry.reference === null ? undefined : foreign.get(entry.reference))
    expect(checkDrop(row('p#T2'), row('p#T1'), 'before', reference)).toMatchObject({ ok: false, reason: expect.stringContaining('per node and kind') })
  })

  it('moves a row before its previous sibling or after its next one, never out of its group', () => {
    expect(siblingMove(rows, row('p#T2'), -1, itemOf)).toMatchObject({ ok: true, other: row('p#T1'), place: 'before' })
    expect(siblingMove(rows, row('p#T1'), 1, itemOf)).toMatchObject({ ok: true, other: row('p#T2'), place: 'after' })
    expect(siblingMove(rows, row('p#T2'), 1, itemOf)).toEqual({ ok: false, reason: 'Already last of its group' })
    expect(siblingMove(rows, row('p#S1'), -1, itemOf)).toEqual({ ok: false, reason: 'Already first of its group' })
  })

  it('gives no grip to the row of a done item, which stays a place to drop beside', () => {
    const done = makeItem({ id: 'T1', path: 'tree/p/T1.md', category: 'done', status: 'done' })
    expect(hasGrip(row('p#T1'), done)).toBe(false)
    expect(hasGrip(row('p#T2'), itemOf(row('p#T2')))).toBe(true)
    expect(checkDrop(row('p#T2'), row('p#T1'), 'after', (entry) => (entry.reference === 'p#T1' ? done : itemOf(entry)))).toMatchObject({ ok: true })
  })
})

describe('stretching Gantt bars', () => {
  it('snaps hours to the nearest size of the node', () => {
    expect(snapSize(12, pricing)).toBe('M')
    expect(snapSize(17, pricing)).toBe('M')
    expect(snapSize(19, pricing)).toBe('L')
    expect(snapSize(0.5, pricing)).toBe('XS')
    expect(snapSize(500, pricing)).toBe('XL')
  })

  it('scales the hours of a bar with its width', () => {
    expect(stretchedHours(12, 120, 120)).toBe(24)
    expect(stretchedHours(12, 120, -90)).toBe(3)
    expect(stretchedHours(12, 120, -300)).toBe(0)
    expect(stretchedHours(12, 0, 50)).toBe(12)
  })

  it('steps to the next or previous size, from the default size when the item has none', () => {
    expect(currentSize({ size: null }, pricing)).toBe('M')
    expect(stepSize({ size: 'S' }, pricing, 1)).toBe('M')
    expect(stepSize({ size: null }, pricing, -1)).toBe('S')
    expect(stepSize({ size: 'XL' }, pricing, 1)).toBeNull()
    expect(stepSize({ size: 'XS' }, pricing, -1)).toBeNull()
  })

  it('resizes open work and an unsplit story or epic', () => {
    expect(stretchRefusal(row('p#T2'), items.get('p#T2'))).toBeNull()
    expect(stretchRefusal(row('p#S2'), items.get('p#S2'))).toBeNull()
    expect(stretchRefusal(row('p#E2'), items.get('p#E2'))).toBeNull()
  })

  it('refuses a split story or epic, done work and work whose testing is all that is left', () => {
    expect(stretchRefusal(row('p#S1'), items.get('p#S1'))).toBe('Its size is the sum of its tasks: resize those instead')
    expect(stretchRefusal(row('p#E1'), items.get('p#E1'))).toBe('Its size is the sum of its stories: resize those instead')
    expect(stretchRefusal(row('p#T1'), items.get('p#T1'))).toBe('Done work keeps its size')
    const testing = buildRows(
      makeRoadmap([makeRelease('0.1', [makeEpic(null, [makeStory(null, [planned('p#T3', 0, 0, 3, { stage: 'testing' })])])])]),
      ['task'],
    )
    expect(stretchRefusal(testing[0] as GanttRow, items.get('p#T3'))).toBe('Its work is done: only its testing remains')
    const waiting = buildRows(makeRoadmap([makeRelease('0.1', [makeEpic(null, [makeStory(null, [makeWork('p#T3', { stage: 'waiting' })])])])]), ['task'])
    expect(stretchRefusal(waiting[0] as GanttRow, items.get('p#T3'))).toBe('Its work is done: it waits for validation')
  })
})
