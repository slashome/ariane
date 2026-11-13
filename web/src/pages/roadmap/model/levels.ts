import { useCallback, useState } from 'react'

export type Level = 'release' | 'epic' | 'story' | 'task'

export const levels: readonly Level[] = ['release', 'epic', 'story', 'task']

export const levelLetters: Record<Level, string> = { release: 'R', epic: 'E', story: 'S', task: 'T' }

export const levelNames: Record<Level, string> = { release: 'Releases', epic: 'Epics', story: 'Stories', task: 'Tasks' }

export function levelRank(level: Level): number {
  return levels.indexOf(level)
}

export function toggleLevel(active: readonly Level[], level: Level): Level[] {
  if (active.includes(level)) {
    return active.length === 1 ? [...active] : active.filter((entry) => entry !== level)
  }
  return levels.filter((entry) => entry === level || active.includes(entry))
}

export function parseLevels(stored: unknown): Level[] {
  if (!Array.isArray(stored)) {
    return [...levels]
  }
  const kept = levels.filter((level) => stored.includes(level))
  return kept.length === 0 ? [...levels] : kept
}

const storageKey = 'ariane.roadmap.levels'

function readLevels(): Level[] {
  try {
    return parseLevels(JSON.parse(localStorage.getItem(storageKey) ?? 'null'))
  } catch {
    return [...levels]
  }
}

function storeLevels(active: Level[]) {
  try {
    localStorage.setItem(storageKey, JSON.stringify(active))
  } catch {
    return
  }
}

export function useGanttLevels(): [Level[], (level: Level) => void] {
  const [active, setActive] = useState(readLevels)
  const toggle = useCallback(
    (level: Level) =>
      setActive((current) => {
        const next = toggleLevel(current, level)
        storeLevels(next)
        return next
      }),
    [],
  )
  return [active, toggle]
}
