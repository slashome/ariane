import { useMemo, useState } from 'react'
import { ItemLink, useItems } from '@/entities/item'
import { checkLabel, countBySeverity, scopeFindings, useDoctor } from '@/entities/doctor'
import { groupBy } from '@/entities/item'
import { useItemFilters } from '@/features/filter-items'
import type { Finding } from '@/shared/api'
import { cn, displayNode } from '@/shared/lib'
import { EmptyState, ErrorState, Icon, LoadingState, NativeSelect, PageHeader, toast } from '@/shared/ui'
import './health.css'

type Grouping = 'node' | 'check'

function Remedy({ finding }: { finding: Finding }) {
  if (finding.remedy.kind === 'command') {
    const { command } = finding.remedy
    return (
      <div className="health-remedy">
        <code className="health-remedy__command">{command}</code>
        <button
          type="button"
          aria-label={`Copy ${command}`}
          className="lu-icon-button health-remedy__copy"
          onClick={() => {
            void navigator.clipboard?.writeText(command)
            toast.success('Command copied')
          }}
        >
          <Icon name="copy" className="health-remedy__copy-icon" />
        </button>
      </div>
    )
  }
  return (
    <ul className="health-remedy__resolutions">
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
    <div className="health-page">
      <PageHeader
        title="Health"
        description="What `ariane doctor` finds, with how to repair it. Doctor never writes: the repairs are commands to run, or a choice to make."
        actions={
          <label className="health-page__grouping">
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
          <dl className="health-page__tiles">
            {[
              { label: 'Errors', value: counts.errors, tone: counts.errors > 0 && 'health-page__tile-value--error' },
              { label: 'Warnings', value: counts.warnings, tone: counts.warnings > 0 && 'health-page__tile-value--warning' },
              { label: 'Nodes concerned', value: new Set(scoped.map((finding) => finding.node)).size, tone: false },
            ].map((tile) => (
              <div key={tile.label} className="health-page__tile">
                <dt className="health-page__tile-label">{tile.label}</dt>
                <dd className={cn('health-page__tile-value', tile.tone)}>{tile.value}</dd>
              </div>
            ))}
          </dl>
          {scoped.length === 0 ? (
            <EmptyState icon="health" title="Healthy">
              Every item {filters.node === null ? 'of the tree' : `of ${displayNode(filters.node)}`} follows the specification.
            </EmptyState>
          ) : (
            groups.map((group) => (
              <section key={group.key} className="health-group">
                <h2 className="health-group__title">
                  <span className={cn(grouping === 'node' && 'health-group__node')}>{group.label}</span>{' '}
                  <span className="health-group__count">({group.items.length})</span>
                </h2>
                <ul className="health-group__findings">
                  {group.items.map((finding, index) => {
                    const item = finding.path === null ? undefined : byPath.get(finding.path)
                    const icon = finding.severity === 'error' ? 'alert' : 'warning'
                    return (
                      <li key={`${finding.path ?? ''}-${finding.check}-${index}`} className="health-finding">
                        <span
                          className={cn(
                            'health-finding__severity',
                            finding.severity === 'error' ? 'health-finding__severity--error' : 'health-finding__severity--warning',
                          )}
                        >
                          <Icon name={icon} className="health-finding__icon" />
                          {finding.severity}
                        </span>
                        <div className="health-finding__body">
                          <p className="health-finding__message">{finding.message}</p>
                          <p className="lu-truncate health-finding__location">
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
