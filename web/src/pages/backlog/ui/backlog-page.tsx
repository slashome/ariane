import { ArrowRightToLine, Inbox } from 'lucide-react'
import { useMemo, useRef, useState, type KeyboardEvent } from 'react'
import { useNavigate, useLocation } from 'react-router'
import { changesFor, itemHref, ItemLink, useItems, useUpdateItem } from '@/entities/item'
import { firstStatusOf, workflowOf, useNodes } from '@/entities/node'
import { enumValues, useSchema } from '@/entities/schema'
import { QueryField, useFilteredItems, useItemFilters } from '@/features/filter-items'
import type { Item } from '@/shared/api'
import { scopeSearch } from '@/shared/config'
import { cn, displayNode } from '@/shared/lib'
import { Button, EmptyState, ErrorState, Highlight, Kbd, LoadingState, NativeSelect, PageHeader, Segmented, toast } from '@/shared/ui'
import { cycle, missingFields, sortBacklog, sorts, type BacklogSort, type TriageField } from '../model/triage'

const fieldKeys: Record<string, TriageField> = { p: 'priority', i: 'impact', s: 'size' }

export function BacklogPage() {
  const { data: items = [], isPending, error } = useItems()
  const { data: nodes = [] } = useNodes()
  const { data: schema } = useSchema()
  const { filters } = useItemFilters()
  const update = useUpdateItem()
  const navigate = useNavigate()
  const { search } = useLocation()
  const [sort, setSort] = useState<BacklogSort>('triage')
  const [focused, setFocused] = useState(0)
  const rows = useRef<(HTMLLIElement | null)[]>([])

  const vocabulary = useMemo<Record<TriageField, string[]>>(
    () => ({ priority: enumValues(schema, 'priority'), impact: enumValues(schema, 'impact'), size: enumValues(schema, 'size') }),
    [schema],
  )

  const { items: tasks, total } = useFilteredItems(items, { kinds: ['task'], categories: ['backlog'] })
  const backlog = useMemo(() => sortBacklog(tasks, sort, vocabulary.priority, vocabulary.size), [tasks, sort, vocabulary])
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

  const onKeyDown = (event: KeyboardEvent<HTMLLIElement>, item: Item, index: number) => {
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

  return (
    <div className="flex flex-col gap-4 p-6">
      <PageHeader
        title="Backlog"
        description={
          isPending
            ? 'Loading…'
            : `${backlog.length} ${backlog.length === 1 ? 'task' : 'tasks'} waiting in a backlog status · ${untriaged} still to estimate`
        }
        actions={
          <>
          <QueryField count={backlog.length} total={total} />
          <label className="flex items-center gap-2 text-sm text-muted-foreground">
            Sort
            <NativeSelect aria-label="Sort the backlog" value={sort} onChange={(event) => setSort(event.target.value as BacklogSort)}>
              {sorts.map((entry) => (
                <option key={entry.value} value={entry.value}>
                  {entry.label}
                </option>
              ))}
            </NativeSelect>
          </label>
          </>
        }
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
        <ul aria-label="Backlog tasks" className="divide-y divide-border rounded-lg border border-border">
          {backlog.map((item, index) => {
            const missing = missingFields(item)
            return (
              <li
                key={item.path}
                ref={(element) => {
                  rows.current[index] = element
                }}
                tabIndex={index === focused ? 0 : -1}
                onFocus={() => setFocused(index)}
                onKeyDown={(event) => onKeyDown(event, item, index)}
                aria-label={`${item.id ?? ''} ${item.title ?? item.path}`}
                className="grid items-center gap-x-4 gap-y-2 px-3 py-2.5 outline-none focus-visible:bg-primary/5 focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset lg:grid-cols-[minmax(0,1fr)_auto]"
              >
                <div className="min-w-0">
                  <div className="flex items-baseline gap-2">
                    <span className="shrink-0 font-mono text-xs text-muted-foreground">
                      <Highlight text={item.id ?? ''} />
                    </span>
                    <ItemLink item={item} className="truncate">
                      <Highlight text={item.title ?? item.path} />
                    </ItemLink>
                  </div>
                  <p className="mt-0.5 flex flex-wrap gap-x-3 text-xs text-muted-foreground">
                    {item.node !== filters.node && <span className="font-mono">{displayNode(item.node)}</span>}
                    {item.created && <span>since {item.created}</span>}
                    {missing.length > 0 && <span className="text-amber-700 dark:text-amber-300">to estimate: {missing.join(', ')}</span>}
                  </p>
                </div>
                <div className="flex flex-wrap items-center gap-3">
                  {(['priority', 'impact', 'size'] as const).map((field) => (
                    <div key={field} className="flex items-center gap-1.5">
                      <span className={cn('w-12 text-right text-[11px] text-muted-foreground', item[field] === null && 'text-amber-700 dark:text-amber-300')}>
                        {field}
                      </span>
                      <Segmented label={`${field} of ${item.id ?? item.path}`} values={vocabulary[field]} value={item[field]} onChange={(value) => set(item, field, value)} focusable={false} />
                    </div>
                  ))}
                  <Button size="sm" variant="outline" tabIndex={-1} onClick={() => plan(item)} title="Move it to the first todo status of its workflow">
                    <ArrowRightToLine className="size-3.5" aria-hidden />
                    Plan
                  </Button>
                </div>
              </li>
            )
          })}
        </ul>
      )}
    </div>
  )
}
