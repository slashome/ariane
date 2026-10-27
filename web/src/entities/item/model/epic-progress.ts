import type { Item } from '@/shared/api'
import { childrenOf, progressOf, type Progress } from './progress'

export interface EpicProgress {
  stories: Item[]
  tasks: Item[]
  // How far its stories have gone, and the tasks of those stories.
  storyProgress: Progress
  taskProgress: Progress
}

// An epic measured twice: by its stories, the items whose `parent` it is, and by their tasks.
export function epicProgress(epic: Item, items: Item[]): EpicProgress {
  const stories = childrenOf(items, epic.reference).filter((item) => item.kind === 'story')
  const tasks = stories.flatMap((story) => childrenOf(items, story.reference))
  return { stories, tasks, storyProgress: progressOf(stories), taskProgress: progressOf(tasks) }
}
