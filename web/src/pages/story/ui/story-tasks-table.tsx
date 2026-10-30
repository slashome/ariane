import { CircleCheck, CircleDashed, Hourglass } from 'lucide-react'
import { idColumn, isClosed, ItemLink, statusColumn, titleColumn } from '@/entities/item'
import { statusLabel, useNodes } from '@/entities/node'
import type { NodeSummary } from '@/shared/api'
import { DataTable, EmptyState, type Column } from '@/shared/ui'
import type { StoryTask } from '../model/story-tasks'

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
      <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
        <CircleCheck className="size-3.5" aria-hidden /> closed
      </span>
    )
  }
  if (readiness === 'ready') {
    return (
      <span className="inline-flex items-center gap-1 text-xs text-green-700 dark:text-green-300">
        <CircleDashed className="size-3.5" aria-hidden /> ready
      </span>
    )
  }
  return (
    <span className="inline-flex items-center gap-1 text-xs text-amber-700 dark:text-amber-300">
      <Hourglass className="size-3.5" aria-hidden /> waiting
    </span>
  )
}

function Blockers({ row, scope }: { row: StoryTask; scope: string }) {
  if (row.blockers.open.length === 0 && row.blockers.missing.length === 0) {
    return <span className="text-xs text-muted-foreground">—</span>
  }
  return (
    <ul className="flex flex-wrap gap-1">
      {row.blockers.open.map((blocker) => (
        <li key={blocker.path}>
          <ItemLink item={blocker} className="rounded bg-amber-500/15 px-1.5 py-0.5 font-mono text-[11px] text-amber-800 dark:text-amber-200">
            {blocker.node === scope ? blocker.id : blocker.reference}
          </ItemLink>
        </li>
      ))}
      {row.blockers.missing.map((reference) => (
        <li key={reference} className="rounded bg-red-500/15 px-1.5 py-0.5 font-mono text-[11px] text-red-700 dark:text-red-300" title="Points to no item">
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
      details: (row) => row.task.node !== scope && <span className="font-mono">{row.task.node}</span>,
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
      className: 'whitespace-nowrap',
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
