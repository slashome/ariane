import { Archive, FolderTree, ListTree, Network } from 'lucide-react'
import { sortNodes, useNodes } from '@/entities/node'
import { displayNode, type Command } from '@/shared/lib'
import { useItemFilters } from '../model/use-item-filters'

// The toggles every view shares, Archived and Sub-nodes, and the scope: the whole tree, or one node.
export function useFilterCommands(): Command[] {
  const { filters, setIncludeArchived, setIncludeDescendants, setNode } = useItemFilters()
  const { data: nodes = [] } = useNodes()
  const toggles: Command[] = [
    {
      id: 'filter:archived',
      group: 'Actions',
      title: filters.includeArchived ? 'Hide archived items' : 'Show archived items',
      icon: Archive,
      keywords: ['archived', 'archives'],
      hint: filters.includeArchived ? 'on' : 'off',
      run: () => setIncludeArchived(!filters.includeArchived),
    },
    {
      id: 'filter:descendants',
      group: 'Actions',
      title: filters.includeDescendants ? 'Leave out the sub-nodes' : 'Include the sub-nodes',
      icon: ListTree,
      keywords: ['sub-nodes', 'descendants', 'children'],
      hint: filters.includeDescendants ? 'on' : 'off',
      run: () => setIncludeDescendants(!filters.includeDescendants),
    },
  ]
  const scopes: Command[] = [
    {
      id: 'scope:all',
      group: 'Nodes',
      title: 'All nodes',
      icon: Network,
      keywords: ['scope', 'tree', 'everything'],
      hint: filters.node === null ? 'current' : undefined,
      run: () => setNode(null),
    },
    ...sortNodes(nodes).map((node) => ({
      id: `scope:${node.path}`,
      group: 'Nodes',
      title: displayNode(node.path),
      icon: FolderTree,
      keywords: ['scope'],
      hint: filters.node === node.path ? 'current' : `${node.items} items`,
      searchOnly: true,
      run: () => setNode(node.path),
    })),
  ]
  return [...toggles, ...scopes]
}
