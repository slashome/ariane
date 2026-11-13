import { edgeOf, revealLeft, rowExtent } from './edges'
import { sampleRoadmap } from './fixtures'
import { buildRows } from './rows'
import { makeScale } from './scale'

const scale = makeScale(10, false)

describe('rowExtent', () => {
  it('spans the segments of a piece of work, or the fragments of a summary', () => {
    const rows = buildRows(sampleRoadmap(), ['epic'])
    const epic = rows.find((row) => row.id === 'E1')
    expect(rowExtent(epic ?? { work: null, fragments: [] })?.end).toBe(70)
    const work = rows.find((row) => row.id === 'E2')
    expect(rowExtent(work ?? { work: null, fragments: [] })).toEqual({ start: 30, end: 45 })
    expect(rowExtent({ work: null, fragments: [] })).toBeNull()
  })
})

describe('edgeOf', () => {
  const extent = { start: 30, end: 45 }

  it('points left when the bars end before the view, right when they start after it', () => {
    expect(edgeOf(extent, { scale, left: 500, width: 400 })).toBe('before')
    expect(edgeOf(extent, { scale, left: -600, width: 400 })).toBe('after')
  })

  it('shows no arrow when a bar is even partly visible, or when there is no bar', () => {
    expect(edgeOf(extent, { scale, left: 0, width: 400 })).toBeNull()
    expect(edgeOf(extent, { scale, left: 440, width: 400 })).toBeNull()
    expect(edgeOf(null, { scale, left: 0, width: 400 })).toBeNull()
  })
})

describe('revealLeft', () => {
  it('brings the bars in with a margin on the side they come from', () => {
    expect(revealLeft({ start: 30, end: 45 }, 'before', { scale, width: 400 })).toBe(260)
    expect(revealLeft({ start: 30, end: 45 }, 'after', { scale, width: 400 })).toBe(90)
  })

  it('shows the start of bars wider than the view', () => {
    expect(revealLeft({ start: 30, end: 200 }, 'after', { scale, width: 400 })).toBe(260)
  })
})
