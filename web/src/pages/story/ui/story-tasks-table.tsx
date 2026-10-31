import { idColumn, isClosed, ItemLink, statusColumn, titleColumn } from '@/entities/item'
import { statusLabel, useNodes } from '@/entities/node'
import type { NodeSummary } from '@/shared/api'
import { DataTable, EmptyState, Icon, type Column } from '@/shared/ui'
import type { StoryTask } from '../model/story-tasks'
import './story.css'

type Readiness = 'ready' | 'waiting' | 'closed'

const readinessOrder: Readiness[] = ['ready', 'waiting', 'closed']

function readinessOf(row: StoryTask): Readiness {
  if (isClosed(row.task.category)) {
    return 'closed'
  }
  return row.ready ? 'ready' : 'waiting'
}

function ReadinessLabel({ row }: { row: StoryTask }) {
  const readiness = readinessOf(row)
  if (readiness === 'closed') {
    return (
      <span className="story-readiness">
        <Icon name="success" className="story-readiness__icon" /> closed
      </span>
    )
  }
  if (readiness === 'ready') {
    return (
      <span className="story-readiness story-readiness--ready">
        <Icon name="pending" className="story-readiness__icon" /> ready
      </span>
    )
  }
  return (
    <span className="story-readiness story-readiness--waiting">
      <Icon name="waiting" className="story-readiness__icon" /> waiting
    </span>
  )
}

function Blockers({ row, scope }: { row: StoryTask; scope: string }) {
  if (row.blockers.open.length === 0 && row.blockers.missing.length === 0) {
    return <span className="story-blockers__none">—</span>
  }
  return (
    <ul className="story-blockers">
      {row.blockers.open.map((blocker) => (
        <li key={blocker.path}>
          <ItemLink item={blocker} className="story-blockers__blocker">
            {blocker.node === scope ? blocker.id : blocker.reference}
          </ItemLink>
        </li>
      ))}
      {row.blockers.missing.map((reference) => (
        <li key={reference} className="story-blockers__blocker story-blockers__blocker--missing" title="Points to no item">
          {reference}
        </li>
      ))}
    </ul>
  )
}

function columnsFor(nodes: NodeSummary[], scope: string): Column<StoryTask>[] {
  const taskOf = (row: StoryTask) => row.task
  return [
    idColumn(taskOf),
    titleColumn(taskOf, {
      header: 'Task',
      details: (row) => row.task.node !== scope && <span className="story-tasks__node">{row.task.node}</span>,
    }),
    statusColumn(taskOf, (task) => statusLabel(nodes, task.node, task.status)),
    {
      id: 'blockers',
      header: 'Waits for',
      sortValue: (row) => row.blockers.open.length + row.blockers.missing.length || null,
      cell: (row) => <Blockers row={row} scope={scope} />,
    },
    {
      id: 'readiness',
      header: 'Can start',
      sortValue: (row) => readinessOrder.indexOf(readinessOf(row)),
      className: 'story-tasks__readiness',
      cell: (row) => <ReadinessLabel row={row} />,
    },
  ]
}

export function StoryTasksTable({ rows, scope, hidden = 0 }: { rows: StoryTask[]; scope: string; hidden?: number }) {
  const { data: nodes = [] } = useNodes()
  if (rows.length === 0) {
    return hidden > 0 ? (
      <EmptyState title="No task of this story matches the search">{hidden} tasks in the story; the search field keeps none of them.</EmptyState>
    ) : (
      <EmptyState title="No task in this story yet">A task joins it with `parent:` set to this story.</EmptyState>
    )
  }
  return <DataTable rows={rows} columns={columnsFor(nodes, scope)} rowKey={(row) => row.task.path} storageKey="story-tasks" label="Tasks of the story" />
}
