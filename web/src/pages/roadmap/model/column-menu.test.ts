import type { Release } from '@/shared/api'
import { columnMenu, moveRequest, placeChoices } from './column-menu'

const first: Release = { name: '0.2', parking: false, target: null }
const second: Release = { name: '0.3', parking: false, target: null }
const parked: Release = { name: 'plus-tard', parking: true, target: null }
const shownOrder = [first, second, parked]

function reasons(release: Release, usedBy: string[] = [], inheritedFrom: string | null = null) {
  return Object.fromEntries(columnMenu(shownOrder, release, usedBy, inheritedFrom).map((entry) => [entry.action, entry.reason]))
}

describe('the column menu', () => {
  it('cannot move a release past the ends nor across the parking ones', () => {
    expect(reasons(first)).toMatchObject({ left: 'Already first', right: null })
    expect(reasons(second)).toMatchObject({ left: null, right: 'Parking releases stay after the scheduled ones' })
    expect(reasons(parked)).toMatchObject({ left: 'Parking releases stay after the scheduled ones', right: 'Already last' })
  })

  it('refuses to remove a release still named, saying by whom', () => {
    expect(reasons(first, ['p#T1', 'p#T2', 'p#T3', 'p#T4']).remove).toBe('Still the release of p#T1, p#T2, p#T3 and 1 more')
    expect(reasons(first).remove).toBeNull()
  })

  it('locks every list change when the list is inherited', () => {
    const menu = columnMenu(shownOrder, first, [], 'p')
    expect(menu.every((entry) => entry.disabled && entry.reason === 'The list is declared in p')).toBe(true)
  })

  it('labels the parking toggle after the current mark', () => {
    const label = (release: Release) => columnMenu(shownOrder, release, [], null).find((entry) => entry.action === 'parking')?.label
    expect(label(first)).toBe('Make it a parking release')
    expect(label(parked)).toBe('Schedule it (not parking)')
  })

  it('moves next to the shown neighbour', () => {
    expect(moveRequest('p', shownOrder, '0.3', 'left')).toEqual({ node: 'p', name: '0.3', place: 'before', other: '0.2' })
    expect(moveRequest('p', shownOrder, '0.2', 'right')).toEqual({ node: 'p', name: '0.2', place: 'after', other: '0.3' })
    expect(moveRequest('p', shownOrder, '0.2', 'left')).toBeNull()
  })
})

describe('the place of a new release', () => {
  it('defaults to last, before the parking releases', () => {
    const [last, ...others] = placeChoices(shownOrder)
    expect(last).toEqual({ value: 'last', label: 'Last, before plus-tard', change: { place: 'before', other: 'plus-tard' } })
    expect(others.map((choice) => choice.label)).toEqual(['First', 'After 0.2'])
  })

  it('goes at the bottom of a list without parking release', () => {
    expect(placeChoices([])).toEqual([{ value: 'last', label: 'Last', change: { place: 'bottom' } }])
  })
})
