import { CircleAlert, Copy, HeartPulse, TriangleAlert } from 'lucide-react'
import { useMemo, useState } from 'react'
import { ItemLink, useItems } from '@/entities/item'
import { checkLabel, countBySeverity, scopeFindings, useDoctor } from '@/entities/doctor'
import { groupBy } from '@/entities/item'
import { useItemFilters } from '@/features/filter-items'
import type { Finding } from '@/shared/api'
import { cn, displayNode } from '@/shared/lib'
import { EmptyState, ErrorState, LoadingState, NativeSelect, PageHeader, toast } from '@/shared/ui'

type Grouping = 'node' | 'check'

function Remedy({ finding }: { finding: Finding }) {
  if (finding.remedy.kind === 'command') {
    const { command } = finding.remedy
    return (
      <div className="flex items-center gap-2">
        <code className="rounded bg-muted px-2 py-0.5 font-mono text-xs">{command}</code>
        <button
          type="button"
          aria-label={`Copy ${command}`}
          className="rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground"
          onClick={() => {
            void navigator.clipboard?.writeText(command)
            toast.success('Command copied')
          }}
        >
          <Copy className="size-3.5" aria-hidden />
        </button>
      </div>
    )
  }
  return (
    <ul className="list-disc space-y-0.5 pl-4 text-xs text-muted-foreground">
      {finding.remedy.resolutions.map((resolution) => (
        <li key={resolution}>{resolution}</li>
      ))}
    </ul>
  )
}

export function HealthPage() {
  const { data: findings = [], isPending, error } = useDoctor()
  const { data: items = [] } = useItems()
  const { filters } = useItemFilters()
  const [grouping, setGrouping] = useState<Grouping>('node')
  const scoped = useMemo(
    () => scopeFindings(findings, filters.node, filters.includeDescendants),
    [findings, filters.node, filters.includeDescendants],
  )
  const counts = countBySeverity(scoped)
  const byPath = useMemo(() => new Map(items.map((item) => [item.path, item])), [items])
  const groups = groupBy(
    scoped,
    (finding) => (grouping === 'node' ? (finding.node === null ? null : displayNode(finding.node)) : checkLabel(finding.check)),
    'Whole tree',
  )

  return (
    <div className="flex flex-col gap-5 p-6">
      <PageHeader
        title="Health"
        description="What `ariane doctor` finds, with how to repair it. Doctor never writes: the repairs are commands to run, or a choice to make."
        actions={
          <label className="flex items-center gap-2 text-sm text-muted-foreground">
            Group by
            <NativeSelect aria-label="Group findings by" value={grouping} onChange={(event) => setGrouping(event.target.value as Grouping)}>
              <option value="node">Node</option>
              <option value="check">Check</option>
            </NativeSelect>
          </label>
        }
      />
      {error ? (
        <ErrorState message={`Could not run doctor: ${error.message}`} />
      ) : isPending ? (
        <LoadingState />
      ) : (
        <>
          <dl className="grid max-w-xl grid-cols-3 gap-3">
            {[
              { label: 'Errors', value: counts.errors, tone: counts.errors > 0 ? 'text-red-700 dark:text-red-300' : '' },
              { label: 'Warnings', value: counts.warnings, tone: counts.warnings > 0 ? 'text-amber-700 dark:text-amber-300' : '' },
              { label: 'Nodes concerned', value: new Set(scoped.map((finding) => finding.node)).size, tone: '' },
            ].map((tile) => (
              <div key={tile.label} className="rounded-lg border border-border p-3">
                <dt className="text-xs text-muted-foreground">{tile.label}</dt>
                <dd className={cn('text-2xl font-semibold tabular-nums', tile.tone)}>{tile.value}</dd>
              </div>
            ))}
          </dl>
          {scoped.length === 0 ? (
            <EmptyState icon={HeartPulse} title="Healthy">
              Every item {filters.node === null ? 'of the tree' : `of ${displayNode(filters.node)}`} follows the specification.
            </EmptyState>
          ) : (
            groups.map((group) => (
              <section key={group.key} className="space-y-2">
                <h2 className="text-sm font-semibold">
                  <span className={grouping === 'node' ? 'font-mono' : ''}>{group.label}</span>{' '}
                  <span className="font-normal text-muted-foreground">({group.items.length})</span>
                </h2>
                <ul className="divide-y divide-border rounded-lg border border-border">
                  {group.items.map((finding, index) => {
                    const item = finding.path === null ? undefined : byPath.get(finding.path)
                    const Icon = finding.severity === 'error' ? CircleAlert : TriangleAlert
                    return (
                      <li key={`${finding.path ?? ''}-${finding.check}-${index}`} className="grid gap-2 p-3 md:grid-cols-[7rem_minmax(0,1fr)_minmax(0,22rem)]">
                        <span
                          className={cn(
                            'inline-flex items-center gap-1 self-start text-xs font-medium',
                            finding.severity === 'error' ? 'text-red-700 dark:text-red-300' : 'text-amber-700 dark:text-amber-300',
                          )}
                        >
                          <Icon className="size-3.5" aria-hidden />
                          {finding.severity}
                        </span>
                        <div className="min-w-0 space-y-0.5 text-sm">
                          <p>{finding.message}</p>
                          <p className="truncate font-mono text-xs text-muted-foreground">
                            {grouping === 'check' ? '' : `${checkLabel(finding.check)} · `}
                            {item ? <ItemLink item={item}>{finding.path}</ItemLink> : finding.path}
                          </p>
                        </div>
                        <Remedy finding={finding} />
                      </li>
                    )
                  })}
                </ul>
              </section>
            ))
          )}
        </>
      )}
    </div>
  )
}
