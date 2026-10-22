import { blockersOf, dependencyOrder, indexByReference, isClosed, type Blockers } from '@/entities/item'
import type { Item } from '@/shared/api'

export interface StoryTask {
  task: Item
  blockers: Blockers
  // Open, and nothing it depends on is still open: it can start now.
  ready: boolean
}

// The tasks of a story in the order their dependencies allow, each with what still blocks it.
// Dependencies are resolved against every item, so a task waiting on another story's task says so.
export function storyTasks(children: Item[], items: Item[]): StoryTask[] {
  const byReference = indexByReference(items)
  return dependencyOrder(children).map((task) => {
    const blockers = blockersOf(task, byReference)
    return { task, blockers, ready: !isClosed(task.category) && blockers.open.length === 0 && task.category !== 'blocked' }
  })
}
