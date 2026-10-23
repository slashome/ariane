import { renderHook, act } from '@testing-library/react'
import { neighbours } from './neighbours'
import { useStoredSet } from './use-stored-set'

describe('neighbours', () => {
  it('give the position and the entries around the current one', () => {
    expect(neighbours(['a', 'b', 'c'], (entry) => entry === 'b')).toEqual({ position: 2, previous: 'a', next: 'c' })
    expect(neighbours(['a', 'b'], (entry) => entry === 'a')).toEqual({ position: 1, previous: undefined, next: 'b' })
    expect(neighbours(['a'], (entry) => entry === 'z')).toBeNull()
  })
})

describe('useStoredSet', () => {
  afterEach(() => localStorage.clear())

  it('remembers what is toggled across mounts', () => {
    const first = renderHook(() => useStoredSet('test.set'))
    act(() => first.result.current[1]('done'))
    expect(first.result.current[0].has('done')).toBe(true)
    first.unmount()
    const second = renderHook(() => useStoredSet('test.set'))
    expect(second.result.current[0].has('done')).toBe(true)
    act(() => second.result.current[1]('done'))
    expect(second.result.current[0].size).toBe(0)
  })

  it('starts empty from corrupt storage', () => {
    localStorage.setItem('test.set', '{nope')
    expect(renderHook(() => useStoredSet('test.set')).result.current[0].size).toBe(0)
  })
})
