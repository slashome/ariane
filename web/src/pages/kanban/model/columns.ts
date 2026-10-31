import { categoryOrder, isWorkItem } from '@/entities/item'
import { firstStatusOf, workflowOf } from '@/entities/node'
import type { Category, Item, NodeSummary, Schema, WorkflowStatus } from '@/shared/api'

export interface KanbanColumn {
  id: string
  label: string
  category: Category
  status: string | null
  items: Item[]
}

function categoryLabel(schema: Schema | undefined, category: Category): string {
  return schema?.categories.find((entry) => entry.id === category)?.label ?? category
}

function placeIn(columns: KanbanColumn[], item: Item): KanbanColumn | undefined {
  return (
    columns.find((column) => column.status !== null && column.status === item.status) ??
    columns.find((column) => column.category === item.category)
  )
}

export function buildColumns(
  items: Item[],
  nodes: NodeSummary[],
  node: string | null,
  schema: Schema | undefined,
): KanbanColumn[] {
  const workflow: WorkflowStatus[] = node === null ? [] : workflowOf(nodes, node)
  const columns: KanbanColumn[] =
    workflow.length > 0
      ? workflow.map((status) => ({
          id: `status:${status.id}`,
          label: status.label,
          category: status.category,
          status: status.id,
          items: [],
        }))
      : categoryOrder.map((category) => ({
          id: `category:${category}`,
          label: categoryLabel(schema, category),
          category,
          status: null,
          items: [],
        }))
  for (const item of items) {
    if (item.archived || !isWorkItem(item)) {
      continue
    }
    placeIn(columns, item)?.items.push(item)
  }
  return columns
}

export type DropTarget = { ok: true; status: string; category: Category } | { ok: false; reason: string }

export function targetStatus(item: Item, column: KanbanColumn, nodes: NodeSummary[]): DropTarget {
  const own = workflowOf(nodes, item.node)
  const exact = column.status === null ? undefined : own.find((entry) => entry.id === column.status)
  const chosen = exact ?? firstStatusOf(own, column.category)
  if (chosen === null) {
    return { ok: false, reason: `${item.id ?? item.path} has no “${column.label}” status in its node workflow` }
  }
  return { ok: true, status: chosen.id, category: chosen.category }
}

export interface Drop {
  column: KanbanColumn
  other: Item | null
  place: 'before' | 'after' | null
}

const samePeer = (item: Item, other: Item) => other.path !== item.path && other.node === item.node && other.kind === item.kind

export function dropOf(item: Item, overId: string, columns: KanbanColumn[]): Drop | null {
  const holder = columns.find((column) => column.items.some((entry) => entry.path === overId))
  if (holder === undefined) {
    const column = columns.find((entry) => entry.id === overId)
    if (column === undefined) {
      return null
    }
    const last = column.items.filter((entry) => samePeer(item, entry)).at(-1)
    return { column, other: last ?? null, place: last === undefined ? null : 'after' }
  }
  const over = holder.items.find((entry) => entry.path === overId)
  if (over === undefined || !samePeer(item, over)) {
    return { column: holder, other: null, place: null }
  }
  const from = holder.items.findIndex((entry) => entry.path === item.path)
  const to = holder.items.indexOf(over)
  return { column: holder, other: over, place: from !== -1 && from < to ? 'after' : 'before' }
}
