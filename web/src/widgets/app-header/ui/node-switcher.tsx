import { useSearchParams } from 'react-router'
import { sortNodes, useNodes } from '@/entities/node'
import { searchParamKeys } from '@/shared/config'
import { displayNode } from '@/shared/lib'
import { NativeSelect } from '@/shared/ui'

const allNodes = '*'

export function NodeSwitcher() {
  const { data: nodes = [] } = useNodes()
  const [params, setParams] = useSearchParams()
  const current = params.get(searchParamKeys.node)

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

  return (
    <NativeSelect
      aria-label="Node"
      className="max-w-72"
      value={current ?? allNodes}
      onChange={(event) => select(event.target.value)}
    >
      <option value={allNodes}>All nodes</option>
      {sortNodes(nodes).map((node) => (
        <option key={node.path} value={node.path}>
          {displayNode(node.path)} ({node.items})
        </option>
      ))}
    </NativeSelect>
  )
}
