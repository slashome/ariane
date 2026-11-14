import { ArrowRight, RotateCcw } from 'lucide-react'
import { useState } from 'react'
import { Link, useLocation } from 'react-router'
import { useNodes } from '@/entities/node'
import type { EstimateKey, NodeSummary, Pricing } from '@/shared/api'
import { routes, scopeSearch } from '@/shared/config'
import { cn, displayNode } from '@/shared/lib'
import { Badge, Input, LoadingState, NativeSelect, toast } from '@/shared/ui'
import { useEstimatesChange } from '../api/use-estimates-change'
import { estimateEdit, estimateKeys, estimateLabels, originOf, resetChange, sizeKeys, testers, type Origin } from '../model/estimates'

const originStyles: Record<Origin['kind'], string> = {
  own: 'bg-primary/15 text-foreground',
  inherited: 'bg-muted text-muted-foreground',
  default: 'border-border text-muted-foreground',
}

const hints: Partial<Record<EstimateKey, string>> = {
  default: 'What an item without size counts as',
  test_ratio: 'Hours of testing after each hour of work',
  tester: 'Owner: testing takes your hours; others: it only delays the release',
}

function EstimateValue({
  name,
  value,
  pending,
  onCommit,
}: {
  name: EstimateKey
  value: string
  pending: boolean
  onCommit: (raw: string) => void
}) {
  const [text, setText] = useState(value)
  if (name === 'default' || name === 'tester') {
    const choices: readonly string[] = name === 'default' ? sizeKeys : testers
    return (
      <NativeSelect aria-label={estimateLabels[name]} value={value} disabled={pending} onChange={(event) => onCommit(event.target.value)} className="h-8 w-28">
        {choices.map((choice) => (
          <option key={choice} value={choice}>
            {choice}
          </option>
        ))}
      </NativeSelect>
    )
  }
  return (
    <Input
      type="number"
      aria-label={estimateLabels[name]}
      min={0}
      step={name === 'test_ratio' ? 0.05 : 1}
      value={text}
      disabled={pending}
      onChange={(event) => setText(event.target.value)}
      onBlur={() => onCommit(text)}
      onKeyDown={(event) => {
        if (event.key === 'Enter') {
          event.currentTarget.blur()
        }
        if (event.key === 'Escape') {
          setText(value)
        }
      }}
      className="h-8 w-28"
    />
  )
}

function EstimatesTable({ node, estimates, from }: { node: string; estimates: Pricing; from: NodeSummary['estimates_from'] }) {
  const change = useEstimatesChange()
  const commit = (key: EstimateKey, raw: string) => {
    const edit = estimateEdit(node, key, raw, estimates)
    if (edit === null) {
      return
    }
    if ('error' in edit) {
      toast.error(edit.error)
      return
    }
    change.mutate(edit.change)
  }
  return (
    <table aria-label="Estimates" className="w-full max-w-2xl text-sm">
      <thead>
        <tr className="text-left text-xs text-muted-foreground">
          <th className="py-1 font-medium">Key</th>
          <th className="py-1 font-medium">Value</th>
          <th className="py-1 font-medium">From</th>
          <th className="py-1" />
        </tr>
      </thead>
      <tbody>
        {estimateKeys.map((key) => {
          const origin = originOf(node, from?.[key])
          const value = String(estimates[key])
          return (
            <tr key={key} className="border-t border-border">
              <th scope="row" className="py-1.5 pr-3 text-left font-medium">
                {estimateLabels[key]}
                {hints[key] && <span className="block text-xs font-normal text-muted-foreground">{hints[key]}</span>}
              </th>
              <td className="py-1.5 pr-3">
                <span className="flex items-center gap-1.5">
                  <EstimateValue key={`${key}:${value}`} name={key} value={value} pending={change.isPending} onCommit={(raw) => commit(key, raw)} />
                  {(sizeKeys as readonly string[]).includes(key) && <span className="text-xs text-muted-foreground">h</span>}
                </span>
              </td>
              <td className="py-1.5 pr-3">
                <Badge className={cn('border', originStyles[origin.kind])} data-testid={`origin-${key}`}>
                  {origin.label}
                </Badge>
              </td>
              <td className="py-1.5 text-right">
                {origin.kind === 'own' && (
                  <button
                    type="button"
                    aria-label={`Reset ${estimateLabels[key]}`}
                    title="Remove this node's value: inherit it again"
                    disabled={change.isPending}
                    onClick={() => change.mutate(resetChange(node, key))}
                    className="rounded p-1 text-muted-foreground hover:bg-muted hover:text-foreground"
                  >
                    <RotateCcw className="size-3.5" aria-hidden />
                  </button>
                )}
              </td>
            </tr>
          )
        })}
      </tbody>
    </table>
  )
}

function ReleaseSummary({ node, summary }: { node: string; summary: NodeSummary }) {
  const { search } = useLocation()
  const releases = summary.releases ?? []
  const declaredIn = summary.releases_from ?? null
  return (
    <div className="space-y-2">
      <h3 className="text-sm font-semibold">Releases</h3>
      {releases.length === 0 ? (
        <p className="text-xs text-muted-foreground">No release declared for {displayNode(node)}.</p>
      ) : (
        <>
          {declaredIn !== null && declaredIn !== node && (
            <p className="text-xs text-muted-foreground">Declared in {displayNode(declaredIn)}, in delivery order.</p>
          )}
          <ol aria-label="Releases" className="flex flex-wrap items-center gap-1.5 text-xs">
            {releases.map((release, index) => (
              <li key={release.name} className="flex items-center gap-1.5">
                {index > 0 && <ArrowRight className="size-3 text-muted-foreground" aria-hidden />}
                <span className={cn('rounded-full border border-border px-2 py-0.5 font-mono', release.parking && 'border-dashed text-muted-foreground')}>
                  {release.name}
                  {release.parking && ' · parking'}
                  {release.target && ` · ${release.target}`}
                </span>
              </li>
            ))}
          </ol>
        </>
      )}
      <Link
        to={{ pathname: routes.releases, search: scopeSearch(search) }}
        className="inline-flex items-center gap-1 text-xs font-medium underline underline-offset-2 hover:text-primary"
      >
        Plan them in Releases
        <ArrowRight className="size-3" aria-hidden />
      </Link>
    </div>
  )
}

export function EstimatesSection({ node }: { node: string | null }) {
  const { data: nodes, isPending } = useNodes()
  if (node === null) {
    return <p className="text-sm text-muted-foreground">Choose a node in the header to set its estimates.</p>
  }
  if (isPending) {
    return <LoadingState rows={3} />
  }
  const summary = nodes?.find((entry) => entry.path === node)
  if (summary?.estimates === undefined) {
    return <p className="text-sm text-muted-foreground">{displayNode(node)} is not a node of the tree.</p>
  }
  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <h3 className="text-sm font-semibold">Estimates</h3>
        <p className="text-xs text-muted-foreground">
          Hours of focused work per size. A value is written in this node's own table at once; reset it to inherit it again.
        </p>
        <EstimatesTable node={node} estimates={summary.estimates} from={summary.estimates_from} />
      </div>
      <ReleaseSummary node={node} summary={summary} />
    </div>
  )
}
