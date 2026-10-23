import { routes } from './routes'

// Every view of the interface, with the words that find it besides its title.
export const views: { path: string; title: string; keywords: string[] }[] = [
  { path: routes.home, title: 'Dashboard', keywords: ['home'] },
  { path: routes.taskList, title: 'Tasks', keywords: ['list', 'table'] },
  { path: routes.kanban, title: 'Kanban', keywords: ['board', 'tasks'] },
  { path: routes.matrix, title: 'Size × impact', keywords: ['matrix', 'estimate', 'tasks'] },
  { path: routes.backlog, title: 'Backlog', keywords: ['triage', 'tasks'] },
  { path: routes.stories, title: 'Stories', keywords: [] },
  { path: routes.reflexions, title: 'Reflexions', keywords: ['board'] },
  { path: routes.journal, title: 'Journal', keywords: ['reflexions', 'settled'] },
  { path: routes.ressources, title: 'Ressources', keywords: ['library', 'links'] },
  { path: routes.people, title: 'People', keywords: ['directory', 'waiting'] },
  { path: routes.health, title: 'Health', keywords: ['doctor', 'findings'] },
  { path: routes.repo, title: 'Repository', keywords: ['git', 'commit', 'diff'] },
  { path: routes.settings, title: 'Settings', keywords: ['theme', 'colors', 'pins'] },
]
