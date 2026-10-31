import { useMemo } from 'react'
import { useLocation, useSearchParams } from 'react-router'
import { useItems } from '@/entities/item'
import { findNode, hasDescendants, nodeTree, useNodes } from '@/entities/node'
import { useItemFilters } from '@/features/filter-items'
import { routes, searchParamKeys } from '@/shared/config'
import { displayNode } from '@/shared/lib'
import { Select, SelectItem, Switch } from '@/shared/ui'
import { nodeCounts } from '../model/node-counts'
import './app-header.css'

const allNodes = '*'
// Radix refuses an empty value, which is the home node's path.
const homeValue = '~'

// A select whose options draw the node tree in monospace, so the tree lines align; its closed face
// shows the full path of the node in scope, which a tree line alone (`└─ cmc-lib`) would not tell.
export function NodeSwitcher() {
  const { data: everyNode = [] } = useNodes()
  const { pathname } = useLocation()
  // On the People page, only the nodes that bind a tracker, where people are declared.
  const trackersOnly = pathname === routes.people
  const nodes = useMemo(
    () => (trackersOnly ? everyNode.filter((node) => node.tracker_from === node.path) : everyNode),
    [everyNode, trackersOnly],
  )
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
    <div className="node-switcher">
      <Select
        label="Node"
        value={current === null ? allNodes : current === '' ? homeValue : current}
        onValueChange={(value) => select(value === homeValue ? '' : value)}
        className="node-switcher__select"
        display={
          <span className="node-switcher__option">
            {current === null ? 'All nodes' : displayNode(current)}
            {count !== undefined && <span className="node-switcher__count"> ({count})</span>}
          </span>
        }
      >
        <SelectItem value={allNodes}>All nodes</SelectItem>
        {home && (
          <SelectItem value={homeValue} textValue="(home)">
            <span className="node-switcher__option">
              (home) <span className="node-switcher__count">({counts.get('') ?? 0})</span>
            </span>
          </SelectItem>
        )}
        {trackersOnly && nodes.length === 0 && (
          <div className="node-switcher__empty">No node binds a tracker</div>
        )}
        {nodeTree(nodes).map((row) => (
          <SelectItem key={row.node.path} value={row.node.path} textValue={row.name}>
            <span className="node-switcher__option node-switcher__option--tree">
              <span className="node-switcher__prefix" aria-hidden>
                {row.prefix}
              </span>
              {row.name} <span className="node-switcher__count">({counts.get(row.node.path) ?? 0})</span>
            </span>
          </SelectItem>
        ))}
      </Select>
      {hasDescendants(everyNode, current) && (
        <label className="node-switcher__descendants" title="Bring the items of the sub-nodes into every view">
          <Switch checked={filters.includeDescendants} onCheckedChange={setIncludeDescendants} aria-label="Include the sub-nodes" />
          Sub-nodes
        </label>
      )}
    </div>
  )
}
