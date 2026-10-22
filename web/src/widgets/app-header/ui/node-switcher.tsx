import { ChevronDown } from 'lucide-react'
import { useMemo } from 'react'
import { useSearchParams } from 'react-router'
import { useItems } from '@/entities/item'
import { findNode, hasDescendants, nodeTree, useNodes } from '@/entities/node'
import { useItemFilters } from '@/features/filter-items'
import { searchParamKeys } from '@/shared/config'
import { displayNode } from '@/shared/lib'
import { Switch } from '@/shared/ui'
import { nodeCounts } from '../model/node-counts'

const allNodes = '*'

// A select whose options draw the node tree; its closed face shows the full path of the node in scope,
// which a tree line alone (`└─ cmc-lib`) would not tell. The native select lies on top, invisible, so
// the keyboard and the option list stay the browser's.
export function NodeSwitcher() {
  const { data: nodes = [] } = useNodes()
  const [params, setParams] = useSearchParams()
  const { data: items = [] } = useItems()
  const { filters, setIncludeDescendants } = useItemFilters()
  const { includeArchived, includeDescendants } = filters
  const counts = useMemo(
    () => nodeCounts(items, nodes.map((node) => node.path), { includeArchived, includeDescendants }),
    [items, nodes, includeArchived, includeDescendants],
  )
  const current = params.get(searchParamKeys.node)
  const home = findNode(nodes, '')

  const select = (value: string) =>
    setParams(
      (previous) => {
        const next = new URLSearchParams(previous)
        if (value === allNodes) {
          next.delete(searchParamKeys.node)
        } else {
          next.set(searchParamKeys.node, value)
        }
        return next
      },
      { replace: true },
    )

  const count = current === null ? undefined : counts.get(current)
  return (
    <div className="flex items-center gap-3">
      <div className="relative inline-flex h-9 max-w-80 min-w-48 items-center rounded-md border border-border bg-background pr-8 pl-2.5 text-sm has-[select:focus-visible]:outline-2 has-[select:focus-visible]:outline-ring">
        <span className="truncate font-mono text-xs" aria-hidden>
          {current === null ? 'All nodes' : displayNode(current)}
          {count !== undefined && <span className="text-muted-foreground"> ({count})</span>}
        </span>
        <ChevronDown className="pointer-events-none absolute top-1/2 right-2.5 size-4 -translate-y-1/2 text-muted-foreground" aria-hidden />
        <select
          aria-label="Node"
          className="absolute inset-0 w-full cursor-pointer appearance-none font-mono opacity-0"
          value={current ?? allNodes}
          onChange={(event) => select(event.target.value)}
        >
          <option value={allNodes}>All nodes</option>
          {home && <option value="">(home) ({counts.get('') ?? 0})</option>}
          {nodeTree(nodes).map((row) => (
            <option key={row.node.path} value={row.node.path}>
              {`${row.prefix.replace(/ /g, '\u00a0')}${row.name} (${counts.get(row.node.path) ?? 0})`}
            </option>
          ))}
        </select>
      </div>
      {hasDescendants(nodes, current) && (
        <label className="flex items-center gap-2 text-sm whitespace-nowrap text-muted-foreground" title="Bring the items of the sub-nodes into every view">
          <Switch checked={filters.includeDescendants} onCheckedChange={setIncludeDescendants} aria-label="Include the sub-nodes" />
          Sub-nodes
        </label>
      )}
    </div>
  )
}
