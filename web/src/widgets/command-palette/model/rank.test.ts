import type { Command } from '@/shared/lib'
import { rankCommands } from './rank'

function command(overrides: Partial<Command> & Pick<Command, 'id' | 'title'>): Command {
  return { group: 'Items', ...overrides }
}

const commands: Command[] = [
  command({ id: 'view:kanban', group: 'Views', title: 'Kanban', keywords: ['board', 'tasks'] }),
  command({ id: 'view:tasks', group: 'Views', title: 'Tasks', keywords: ['list'] }),
  command({ id: 'T2', title: 'Écrire la doc des tâches', keywords: ['T002', 'p#T002'], searchOnly: true }),
  command({ id: 'T1', title: 'Préparer le kanban', keywords: ['T001', 'p#T001'], searchOnly: true }),
  command({ id: 'T3', title: 'Revoir T002', keywords: ['T003', 'p#T003'], searchOnly: true }),
  command({ id: 'hit', group: 'In the text', title: 'Found by the server', matched: true }),
]

const ids = (text: string, limit?: number) =>
  rankCommands(commands, text, limit).map((section) => [section.group, section.entries.map((entry) => entry.command.id)])

describe('rankCommands', () => {
  it('lists what is not kept for searching when nothing is typed', () => {
    expect(ids('')).toEqual([
      ['Views', ['view:kanban', 'view:tasks']],
      ['In the text', ['hit']],
    ])
  })

  it('ranks an exact id or reference first, then a title that starts with the text, then word starts', () => {
    expect(ids('t002')).toEqual([
      ['Items', ['T2', 'T3']],
      ['In the text', ['hit']],
    ])
    expect(ids('kan')).toEqual([
      ['Views', ['view:kanban']],
      ['Items', ['T1']],
      ['In the text', ['hit']],
    ])
    expect(ids('taches doc')).toEqual([
      ['Items', ['T2']],
      ['In the text', ['hit']],
    ])
  })

  it('finds by keyword, keeps at most `limit` per section, and highlights the title', () => {
    expect(ids('board')).toEqual([
      ['Views', ['view:kanban']],
      ['In the text', ['hit']],
    ])
    expect(ids('e', 1)).toEqual([
      ['Items', ['T2']],
      ['In the text', ['hit']],
    ])
    const [section] = rankCommands(commands, 'prepa kanban')
    expect(section?.entries[0]?.ranges).toEqual([[0, 5], [12, 18]])
  })
})
