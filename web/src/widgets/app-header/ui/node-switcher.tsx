import { useMemo } from 'react'
import { useSearchParams } from 'react-router'
import { useItems } from '@/entities/item'
import { findNode, hasDescendants, nodeTree, useNodes } from '@/entities/node'
import { useItemFilters } from '@/features/filter-items'
import { searchParamKeys } from '@/shared/config'
import { displayNode } from '@/shared/lib'
import { Select, SelectItem, Switch } from '@/shared/ui'
import { nodeCounts } from '../model/node-counts'

const allNodes = '*'
// Radix refuses an empty value, which is the home node's path.
const homeValue = '~'

// A select whose options draw the node tree in monospace, so the tree lines align; its closed face
// shows the full path of the node in scope, which a tree line alone (`└─ cmc-lib`) would not tell.
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
      <Select
        label="Node"
        value={current === null ? allNodes : current === '' ? homeValue : current}
        onValueChange={(value) => select(value === homeValue ? '' : value)}
        className="w-72"
        display={
          <span className="font-mono text-xs">
            {current === null ? 'All nodes' : displayNode(current)}
            {count !== undefined && <span className="text-muted-foreground"> ({count})</span>}
          </span>
        }
      >
        <SelectItem value={allNodes}>All nodes</SelectItem>
        {home && (
          <SelectItem value={homeValue} textValue="(home)">
            <span className="font-mono text-xs">
              (home) <span className="text-muted-foreground">({counts.get('') ?? 0})</span>
            </span>
          </SelectItem>
        )}
        {nodeTree(nodes).map((row) => (
          <SelectItem key={row.node.path} value={row.node.path} textValue={row.name}>
            <span className="font-mono text-xs whitespace-pre">
              <span className="text-muted-foreground/60" aria-hidden>
                {row.prefix}
              </span>
              {row.name} <span className="text-muted-foreground">({counts.get(row.node.path) ?? 0})</span>
            </span>
          </SelectItem>
        ))}
      </Select>
      {hasDescendants(nodes, current) && (
        <label className="flex items-center gap-2 text-sm whitespace-nowrap text-muted-foreground" title="Bring the items of the sub-nodes into every view">
          <Switch checked={filters.includeDescendants} onCheckedChange={setIncludeDescendants} aria-label="Include the sub-nodes" />
          Sub-nodes
        </label>
      )}
    </div>
  )
}
