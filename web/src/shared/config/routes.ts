import type { Kind } from '@/shared/api'

export const routes = {
  home: '/',
  tasks: '/tasks',
  taskList: '/tasks/list',
  kanban: '/tasks/kanban',
  matrix: '/tasks/matrix',
  backlog: '/tasks/backlog',
  stories: '/stories',
  reflexions: '/reflexions',
  journal: '/reflexions/journal',
  ressources: '/ressources',
  health: '/health',
  repo: '/repo',
  proxy: '/proxy',
  settings: '/settings',
  people: '/people',
} as const

const sections: Record<Kind, string> = {
  task: routes.tasks,
  story: routes.stories,
  reflexion: routes.reflexions,
  ressource: routes.ressources,
  unknown: routes.tasks,
}

// The home node has an empty path; `~` stands for it in a URL, where an empty segment would vanish.
const homeSegment = '~'

export function sectionOf(kind: Kind): string {
  return sections[kind]
}

export function itemPath(kind: Kind, node: string, id: string): string {
  const segments = node === '' ? [homeSegment] : node.split('/')
  return [sections[kind], ...segments.map(encodeURIComponent), encodeURIComponent(id)].join('/')
}

// Reads the `<node…>/<id>` tail of an item URL back into the reference `<node>#<id>`.
export function referenceFromSplat(splat: string | undefined): string | null {
  const segments = (splat ?? '').split('/').filter((segment) => segment !== '').map(decodeURIComponent)
  if (segments.length < 2) {
    return null
  }
  const id = segments[segments.length - 1]
  const node = segments.slice(0, -1)
  const path = node.length === 1 && node[0] === homeSegment ? '' : node.join('/')
  return `${path}#${id}`
}
