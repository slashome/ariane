import { ArrowRightToLine, Inbox } from 'lucide-react'
import { useMemo, useRef, useState, type KeyboardEvent } from 'react'
import { useNavigate, useLocation } from 'react-router'
import { changesFor, idColumn, itemHref, titleColumn, useItems, useUpdateItem } from '@/entities/item'
import { firstStatusOf, workflowOf, useNodes } from '@/entities/node'
import { enumValues, useSchema } from '@/entities/schema'
import { QueryField, useFilteredItems, useItemFilters } from '@/features/filter-items'
import type { Item } from '@/shared/api'
import { scopeSearch } from '@/shared/config'
import { displayNode } from '@/shared/lib'
import { Button, DataTable, EmptyState, ErrorState, Kbd, LoadingState, PageHeader, Segmented, toast, type Column } from '@/shared/ui'
import { cycle, level, missingFields, triageFields, triageOrder, type TriageField } from '../model/triage'

const fieldKeys: Record<string, TriageField> = { p: 'priority', i: 'impact', s: 'size' }

export function BacklogPage() {
  const { data: items = [], isPending, error } = useItems()
  const { data: nodes = [] } = useNodes()
  const { data: schema } = useSchema()
  const { filters } = useItemFilters()
  const update = useUpdateItem()
  const navigate = useNavigate()
  const { search } = useLocation()
  const [focused, setFocused] = useState(0)
  const rows = useRef<(HTMLTableRowElement | null)[]>([])

  const vocabulary = useMemo<Record<TriageField, string[]>>(
    () => ({ priority: enumValues(schema, 'priority'), impact: enumValues(schema, 'impact'), size: enumValues(schema, 'size') }),
    [schema],
  )

  const { items: tasks, total } = useFilteredItems(items, { kinds: ['task'], categories: ['backlog'] })
  const backlog = useMemo(() => triageOrder(tasks, vocabulary.priority, vocabulary.size), [tasks, vocabulary])
  const untriaged = backlog.filter((item) => missingFields(item).length > 0).length

  const set = (item: Item, field: TriageField, value: string | null) =>
    update.mutate({ item, changes: changesFor(item, { [field]: value }), optimistic: { [field]: value } })

  const plan = (item: Item) => {
    const target = firstStatusOf(workflowOf(nodes, item.node), 'todo')
    if (target === null) {
      toast.error(`The workflow of ${displayNode(item.node)} has no status in the todo category`)
      return
    }
    update.mutate({ item, changes: changesFor(item, { status: target.id }), optimistic: { status: target.id, category: 'todo' } })
    toast.success(`${item.id ?? 'Item'} planned: ${target.label}`)
  }

  const focus = (index: number) => {
    const next = Math.max(0, Math.min(backlog.length - 1, index))
    setFocused(next)
    rows.current[next]?.focus()
  }

  const onKeyDown = (event: KeyboardEvent<HTMLTableRowElement>, item: Item, index: number) => {
    if (event.metaKey || event.ctrlKey || event.altKey) {
      return
    }
    const field = fieldKeys[event.key]
    if (field) {
      event.preventDefault()
      set(item, field, cycle(vocabulary[field], item[field]))
    } else if (event.key === 'ArrowDown' || event.key === 'j') {
      event.preventDefault()
      focus(index + 1)
    } else if (event.key === 'ArrowUp' || event.key === 'k') {
      event.preventDefault()
      focus(index - 1)
    } else if (event.key === 'a') {
      event.preventDefault()
      plan(item)
    } else if (event.key === 'Enter') {
      const href = itemHref(item)
      if (href) {
        navigate({ pathname: href, search: scopeSearch(search) })
      }
    }
  }

  const itself = (item: Item) => item
  const fieldColumn = (field: TriageField): Column<Item> => ({
    id: field,
    header: field.charAt(0).toUpperCase() + field.slice(1),
    sortValue: (item) => (item[field] === null ? null : level(vocabulary[field], item[field])),
    className: 'align-middle',
    cell: (item) => (
      <Segmented
        label={`${field} of ${item.id ?? item.path}`}
        values={vocabulary[field]}
        value={item[field]}
        onChange={(value) => set(item, field, value)}
        focusable={false}
      />
    ),
  })
  const columns: Column<Item>[] = [
    idColumn(itself),
    titleColumn(itself, {
      header: 'Task',
      details: (item) => {
        const missing = missingFields(item)
        return (
          <>
            {item.node !== filters.node && <span className="font-mono">{displayNode(item.node)}</span>}
            {missing.length > 0 && <span className="text-amber-700 dark:text-amber-300">to estimate: {missing.join(', ')}</span>}
          </>
        )
      },
    }),
    ...triageFields.map(fieldColumn),
    {
      id: 'created',
      header: 'Created',
      sortValue: (item) => item.created,
      className: 'font-mono text-xs whitespace-nowrap text-muted-foreground',
      cell: (item) => item.created ?? '—',
    },
    {
      id: 'plan',
      header: '',
      className: 'align-middle',
      cell: (item) => (
        <Button size="sm" variant="outline" tabIndex={-1} onClick={() => plan(item)} title="Move it to the first todo status of its workflow">
          <ArrowRightToLine className="size-3.5" aria-hidden />
          Plan
        </Button>
      ),
    },
  ]

  return (
    <div className="flex flex-col gap-4 p-6">
      <PageHeader
        title="Backlog"
        description={
          isPending
            ? 'Loading…'
            : `${backlog.length} ${backlog.length === 1 ? 'task' : 'tasks'} waiting in a backlog status · ${untriaged} still to estimate`
        }
        actions={<QueryField count={backlog.length} total={total} />}
      />
      <p className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground">
        <span>
          <Kbd>j</Kbd> <Kbd>k</Kbd> move
        </span>
        <span>
          <Kbd>p</Kbd> priority · <Kbd>i</Kbd> impact · <Kbd>s</Kbd> size
        </span>
        <span>
          <Kbd>a</Kbd> plan it (first todo status)
        </span>
        <span>
          <Kbd>Enter</Kbd> open
        </span>
      </p>
      {error ? (
        <ErrorState message={`Could not load the items: ${error.message}`} />
      ) : isPending ? (
        <LoadingState />
      ) : backlog.length === 0 ? (
        <EmptyState icon={Inbox} title={total === 0 ? 'The backlog is empty' : 'No backlog task matches the search'}>
          {total === 0
            ? "A task joins it when its status belongs to the backlog category of its node's workflow."
            : `${total} backlog tasks in scope; the search field keeps none of them.`}
        </EmptyState>
      ) : (
        <DataTable
          rows={backlog}
          columns={columns}
          rowKey={(item) => item.path}
          storageKey="backlog"
          label="Backlog tasks"
          rowProps={(item, index) => ({
            ref: (element: HTMLTableRowElement | null) => {
              rows.current[index] = element
            },
            tabIndex: index === focused ? 0 : -1,
            onFocus: () => setFocused(index),
            onKeyDown: (event) => onKeyDown(event, item, index),
            'aria-label': `${item.id ?? ''} ${item.title ?? item.path}`,
            className: 'outline-none focus-visible:bg-primary/5 focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset',
          })}
        />
      )}
    </div>
  )
}
