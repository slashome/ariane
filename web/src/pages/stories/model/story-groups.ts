import { categoryOrder, childrenOf, compareNatural, groupBy, progressOf, type Group, type Progress } from '@/entities/item'
import type { Item } from '@/shared/api'

export type StoryGrouping = 'milestone' | 'release' | 'node' | 'status'

export const groupings: { value: StoryGrouping; label: string }[] = [
  { value: 'milestone', label: 'Release › milestone' },
  { value: 'release', label: 'Release' },
  { value: 'node', label: 'Node' },
  { value: 'status', label: 'Status' },
]

export interface StoryRow {
  story: Item
  tasks: Item[]
  progress: Progress
}

export interface StoryGroup extends Group<StoryRow> {
  progress: Progress
}

function keyOf(grouping: StoryGrouping, story: Item): string | null {
  switch (grouping) {
    case 'milestone':
      return story.release === null && story.milestone === null
        ? null
        : [story.release ?? '—', story.milestone].filter((part) => part !== null).join(' › ')
    case 'release':
      return story.release
    case 'node':
      return story.node === '' ? '(home)' : story.node
    case 'status':
      return story.category
  }
}

const noneLabels: Record<StoryGrouping, string> = {
  milestone: 'No release or milestone',
  release: 'No release',
  node: '(home)',
  status: 'No status',
}

const rank = (story: Item) => (story.category === null ? categoryOrder.length : categoryOrder.indexOf(story.category))

// Each story with its tasks — the items whose `parent` is the story — and their progress, grouped.
// Within a group, live stories come first in workflow order, then by id.
export function groupStories(stories: Item[], tasks: Item[], grouping: StoryGrouping, label = (key: string) => key): StoryGroup[] {
  const rows = stories
    .map((story) => {
      const children = childrenOf(tasks, story.reference)
      return { story, tasks: children, progress: progressOf(children) }
    })
    .sort((left, right) => {
      const byStatus = grouping === 'status' ? 0 : rank(left.story) - rank(right.story)
      return byStatus !== 0 ? byStatus : compareNatural(left.story.id ?? '', right.story.id ?? '')
    })
  const groups = groupBy(rows, (row) => keyOf(grouping, row.story), noneLabels[grouping], label)
  if (grouping === 'status') {
    groups.sort((left, right) => categoryOrder.indexOf(left.key as never) - categoryOrder.indexOf(right.key as never))
  }
  return groups.map((group) => ({ ...group, progress: progressOf(group.items.flatMap((row) => row.tasks)) }))
}
