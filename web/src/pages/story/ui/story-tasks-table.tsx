import { CircleCheck, CircleDashed, Hourglass } from 'lucide-react'
import { isClosed, ItemLink, StatusBadge } from '@/entities/item'
import { statusLabel, useNodes } from '@/entities/node'
import { cn } from '@/shared/lib'
import { EmptyState, Highlight } from '@/shared/ui'
import type { StoryTask } from '../model/story-tasks'

function Readiness({ row }: { row: StoryTask }) {
  if (isClosed(row.task.category)) {
    return (
      <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
        <CircleCheck className="size-3.5" aria-hidden /> closed
      </span>
    )
  }
  if (row.ready) {
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

// `hidden` counts the tasks the search field leaves out.
export function StoryTasksTable({ rows, scope, hidden = 0 }: { rows: StoryTask[]; scope: string; hidden?: number }) {
  const { data: nodes = [] } = useNodes()
  if (rows.length === 0) {
    return hidden > 0 ? (
      <EmptyState title="No task of this story matches the search">{hidden} tasks in the story; the search field keeps none of them.</EmptyState>
    ) : (
      <EmptyState title="No task in this story yet">A task joins it with `parent:` set to this story.</EmptyState>
    )
  }
  return (
    <div className="overflow-x-auto rounded-lg border border-border">
      <table className="w-full text-sm">
        <thead className="bg-muted/50 text-left text-xs text-muted-foreground">
          <tr>
            <th className="px-3 py-2 font-medium">Task</th>
            <th className="px-3 py-2 font-medium">Status</th>
            <th className="px-3 py-2 font-medium">Waits for</th>
            <th className="px-3 py-2 font-medium">Can start</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row.task.path} className="border-t border-border align-top">
              <td className={cn('px-3 py-2', isClosed(row.task.category) && 'text-muted-foreground')}>
                <span className="mr-2 font-mono text-xs text-muted-foreground">
                  <Highlight text={row.task.id ?? ''} />
                </span>
                <ItemLink item={row.task}>
                  <Highlight text={row.task.title ?? row.task.path} />
                </ItemLink>
                {row.task.node !== scope && <span className="ml-1 font-mono text-[10px] text-muted-foreground">{row.task.node}</span>}
              </td>
              <td className="px-3 py-2">
                <StatusBadge status={row.task.status} category={row.task.category} label={statusLabel(nodes, row.task.node, row.task.status)} />
              </td>
              <td className="px-3 py-2">
                {row.blockers.open.length === 0 && row.blockers.missing.length === 0 ? (
                  <span className="text-xs text-muted-foreground">—</span>
                ) : (
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
                )}
              </td>
              <td className="px-3 py-2">
                <Readiness row={row} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
