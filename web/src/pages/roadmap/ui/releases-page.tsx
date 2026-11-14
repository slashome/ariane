import { useQueryClient } from '@tanstack/react-query'
import { Plus, SquareKanban } from 'lucide-react'
import { useMemo, useState } from 'react'
import { changesFor, useItems, useUpdateItem } from '@/entities/item'
import { useNodes } from '@/entities/node'
import { useItemFilters } from '@/features/filter-items'
import { queryKeys, type Item, type Release, type ReleaseChange } from '@/shared/api'
import { searchParamKeys } from '@/shared/config'
import { displayNode, useSearchParam } from '@/shared/lib'
import { Button, ConfirmDialog, DragArea, EmptyState, PageHeader, type DragEndEvent } from '@/shared/ui'
import { useCloseRelease } from '../api/use-release-close'
import { useReleaseChange } from '../api/use-release-change'
import { useRoadmap } from '../api/use-roadmap'
import { columnMenu, moveRequest, type ColumnAction } from '../model/column-menu'
import {
  assignment,
  backlogItems,
  boardList,
  columnItems,
  columnSummary,
  defaultBacklogFilters,
  displayOrder,
  outline,
  planningItems,
  releaseUsers,
  typesOf,
} from '../model/release-board'
import { BacklogPanel } from './backlog-panel'
import { CloseReleaseDialog } from './close-release-dialog'
import { InheritedNotice } from './inherited-notice'
import { ReleaseColumn } from './release-column'
import { NewReleaseDialog, RenameReleaseDialog } from './release-dialogs'
import { ReleaseCard, ReleaseOutline } from './release-outline'

function Board({ node }: { node: string }) {
  const queryClient = useQueryClient()
  const { data: nodes = [] } = useNodes()
  const { data: items = [] } = useItems()
  const { data: roadmap } = useRoadmap(node)
  const update = useUpdateItem()
  const releaseChange = useReleaseChange()
  const releaseClose = useCloseRelease()
  const [, setPreview] = useSearchParam(searchParamKeys.preview)
  const [filters, setFilters] = useState(defaultBacklogFilters)
  const [creating, setCreating] = useState(false)
  const [renaming, setRenaming] = useState<Release | null>(null)
  const [removing, setRemoving] = useState<Release | null>(null)
  const [closing, setClosing] = useState<Release | null>(null)

  const list = useMemo(() => boardList(nodes, node), [nodes, node])
  const listNode = list.declaredIn ?? node
  const inheritedFrom = list.inherited ? list.declaredIn : null
  const shownOrder = useMemo(() => displayOrder(list.releases), [list.releases])
  const known = useMemo(
    () => new Map(items.flatMap((item): [string, Item][] => (item.reference === null ? [] : [[item.reference, item]]))),
    [items],
  )
  const planned = useMemo(() => planningItems(items, nodes, node, list.declaredIn), [items, nodes, node, list.declaredIn])
  const backlog = useMemo(() => backlogItems(planned, filters), [planned, filters])
  const types = useMemo(() => typesOf(planned), [planned])

  const send = (action: 'add' | 'change' | 'remove', change: ReleaseChange, onDone?: () => void) =>
    releaseChange.mutate({ action, change }, { onSuccess: onDone })
  const open = (item: Item) => setPreview(item.path)

  const act = (release: Release, action: ColumnAction) => {
    switch (action) {
      case 'rename':
        setRenaming(release)
        return
      case 'left':
      case 'right': {
        const change = moveRequest(listNode, shownOrder, release.name, action)
        if (change) {
          send('change', change)
        }
        return
      }
      case 'parking':
        send('change', { node: listNode, name: release.name, parking: !release.parking })
        return
      case 'close':
        setClosing(release)
        return
      case 'remove':
        setRemoving(release)
        return
    }
  }

  const onDragEnd = ({ active, over }: DragEndEvent) => {
    const item = planned.find((entry) => entry.path === active.id)
    const change = item && over ? assignment(item, String(over.id), list.releases) : null
    if (!item || !change) {
      return
    }
    update.mutate(
      { item, changes: changesFor(item, { release: change.release }), optimistic: change.optimistic },
      { onSettled: () => void queryClient.invalidateQueries({ queryKey: queryKeys.roadmap(node) }) },
    )
  }

  return (
    <>
      <PageHeader
        title="Releases"
        description={`${displayNode(node)} — ${list.releases.length === 0 ? 'no release declared yet' : `releases of ${displayNode(listNode)}, in delivery order`}`}
        actions={
          <Button
            size="sm"
            onClick={() => setCreating(true)}
            disabled={inheritedFrom !== null}
            title={inheritedFrom === null ? 'Declare a release' : `The list is declared in ${displayNode(inheritedFrom)}`}
          >
            <Plus className="size-4" aria-hidden />
            New release
          </Button>
        }
      />
      {inheritedFrom !== null && <InheritedNotice node={node} declaredIn={inheritedFrom} />}
      <DragArea
        onDragEnd={onDragEnd}
        renderOverlay={(id) => {
          const item = planned.find((entry) => entry.path === id)
          return item ? <ReleaseCard item={item} showNode={item.node !== node} className="w-64 shadow-lg" /> : null
        }}
      >
        <div className="flex min-h-0 flex-1 gap-3">
          <BacklogPanel filters={filters} types={types} count={backlog.length} onFilters={setFilters}>
            <ReleaseOutline
              entries={outline(backlog, known)}
              scope={node}
              onOpen={open}
              empty={filters.show === 'unplanned' ? 'Every open item is planned' : 'Nothing open matches'}
            />
          </BacklogPanel>
          <div className="flex min-w-0 flex-1 gap-3 overflow-x-auto pb-2">
            {shownOrder.map((release) => {
              const cards = columnItems(planned, release.name)
              return (
                <ReleaseColumn
                  key={release.name}
                  release={release}
                  summary={columnSummary(roadmap, release, cards)}
                  entries={outline(cards, known)}
                  scope={node}
                  menu={columnMenu(shownOrder, release, releaseUsers(items, nodes, list.declaredIn, release.name), inheritedFrom)}
                  editable={inheritedFrom === null}
                  onAction={(action) => act(release, action)}
                  onTarget={(target) => send('change', { node: listNode, name: release.name, target })}
                  onOpen={open}
                />
              )
            })}
            {shownOrder.length === 0 && (
              <EmptyState icon={SquareKanban} title="No releases" className="w-80 shrink-0 self-start">
                {inheritedFrom === null ? 'New release starts the list of this node, in its node.toml.' : null}
              </EmptyState>
            )}
          </div>
        </div>
      </DragArea>
      <NewReleaseDialog
        key={creating ? 'open' : 'closed'}
        open={creating}
        node={listNode}
        shownOrder={shownOrder}
        pending={releaseChange.isPending}
        onClose={() => setCreating(false)}
        onCreate={(change) => send('add', change, () => setCreating(false))}
      />
      <RenameReleaseDialog
        key={renaming?.name ?? 'none'}
        release={renaming}
        pending={releaseChange.isPending}
        onClose={() => setRenaming(null)}
        onRename={(name) => renaming && send('change', { node: listNode, name: renaming.name, rename_to: name }, () => setRenaming(null))}
      />
      <CloseReleaseDialog
        key={closing?.name ?? 'none'}
        target={closing === null ? null : { node: listNode, name: closing.name }}
        pending={releaseClose.isPending}
        onClose={() => setClosing(null)}
        onConfirm={(carryOver) =>
          closing &&
          releaseClose.mutate({ target: { node: listNode, name: closing.name }, carryOver }, { onSuccess: () => setClosing(null) })
        }
      />
      <ConfirmDialog
        open={removing !== null}
        onOpenChange={(next) => !next && setRemoving(null)}
        title={`Remove ${removing?.name ?? ''}`}
        description={`It is removed from the node.toml of ${displayNode(listNode)}. No item is planned in it.`}
        confirmLabel="Remove"
        pending={releaseChange.isPending}
        onConfirm={() => removing && send('remove', { node: listNode, name: removing.name }, () => setRemoving(null))}
      />
    </>
  )
}

export function ReleasesPage() {
  const { filters } = useItemFilters()
  const node = filters.node
  return (
    <div className="flex h-full flex-col gap-3 p-6">
      {node === null ? (
        <>
          <PageHeader title="Releases" description="Choose a node in the header to plan its releases" />
          <EmptyState icon={SquareKanban} title="No node chosen" />
        </>
      ) : (
        <Board node={node} />
      )}
    </div>
  )
}
