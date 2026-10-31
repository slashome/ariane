import { ChevronRight, ExternalLink, GitBranch, GitPullRequest, type LucideIcon } from 'lucide-react'
import type { ReactNode } from 'react'
import { Link, useLocation } from 'react-router'
import { StatusBadge, useItem, useItems, waitingSummary } from '@/entities/item'
import { externalUrl, peopleOf, statusLabel, trackerOf, useNodes } from '@/entities/node'
import { kindLabel } from '@/entities/schema'
import type { ItemDetail, NodeSummary } from '@/shared/api'
import { scopeSearch, sectionOf } from '@/shared/config'
import { displayNode } from '@/shared/lib'
import { Badge, EmptyState, ErrorState, LoadingState } from '@/shared/ui'
import { branchLink, pullRequestLink, type CodeLink } from '../model/code-links'
import { customEntries, refsOf } from '../model/header-keys'
import { BodyEditor } from './body-editor'
import { ItemEditors } from '@/features/edit-item'
import { RelatedList, Unresolved } from './related-list'

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="grid grid-cols-[6.5rem_1fr] gap-2 text-sm">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="min-w-0 break-words">{children ?? <span className="text-muted-foreground">—</span>}</dd>
    </div>
  )
}

function Externals({ detail, nodes }: { detail: ItemDetail; nodes: NodeSummary[] }) {
  if (detail.external.length === 0) {
    return null
  }
  const tracker = trackerOf(nodes, detail.node)
  return (
    <ul className="space-y-0.5">
      {detail.external.map((entry) => {
        const url = externalUrl(tracker, entry)
        return (
          <li key={entry} className="font-mono text-xs">
            {url ? (
              <a href={url} target="_blank" rel="noreferrer" className="inline-flex items-center gap-1 text-primary hover:underline">
                {entry}
                <ExternalLink className="size-3" aria-hidden />
              </a>
            ) : (
              entry
            )}
          </li>
        )
      })}
    </ul>
  )
}

function CodeLinks({ title, icon: Icon, links }: { title: string; icon: LucideIcon; links: CodeLink[] }) {
  if (links.length === 0) {
    return null
  }
  return (
    <section className="space-y-1.5">
      <h3 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
        {title} <span className="font-normal">({links.length})</span>
      </h3>
      <ul className="space-y-1">
        {links.map((link) => (
          <li key={link.label} className="flex items-start gap-1.5 font-mono text-xs">
            <Icon className="mt-0.5 size-3.5 shrink-0 text-muted-foreground" aria-hidden />
            {link.url ? (
              <a href={link.url} target="_blank" rel="noreferrer" className="min-w-0 break-all text-primary hover:underline">
                {link.label}
              </a>
            ) : (
              <span className="min-w-0 break-all">{link.label}</span>
            )}
          </li>
        ))}
      </ul>
    </section>
  )
}

// The page of one item: its body to read on the left, its header and relations on the right. `children`
// adds what a kind shows on top of the common view, such as the progress of a story.
export function ItemView({
  reference,
  sectionLabel,
  children,
}: {
  reference: string | null
  sectionLabel: string
  children?: (detail: ItemDetail) => ReactNode
}) {
  const { data: detail, isPending, error } = useItem(reference)
  const { data: items = [] } = useItems()
  const { data: nodes = [] } = useNodes()
  const { search } = useLocation()

  if (reference === null) {
    return <EmptyState icon="file-unknown" title="Not an item address" />
  }
  if (error) {
    return <ErrorState message={`Could not load ${reference}: ${error.message}`} />
  }
  if (isPending) {
    return <LoadingState rows={6} label={`Loading ${reference}`} />
  }
  const custom = customEntries(detail.header)
  const refs = refsOf(detail.header)
  const { relations } = detail
  return (
    <article className="flex flex-col gap-5">
      <nav aria-label="Breadcrumb" className="flex flex-wrap items-center gap-1 text-xs text-muted-foreground">
        <Link to={{ pathname: sectionOf(detail.kind), search: scopeSearch(search) }} className="hover:text-foreground">
          {sectionLabel}
        </Link>
        <ChevronRight className="size-3" aria-hidden />
        <span className="font-mono">{displayNode(detail.node)}</span>
        <ChevronRight className="size-3" aria-hidden />
        <span className="font-mono text-foreground">{detail.id}</span>
      </nav>
      <header className="space-y-2">
        <h1 className="text-2xl leading-tight font-semibold">{detail.title ?? detail.path}</h1>
        <div className="flex flex-wrap items-center gap-2">
          <StatusBadge status={detail.status} category={detail.category} label={statusLabel(nodes, detail.node, detail.status)} />
          <Badge className="bg-muted text-muted-foreground">{kindLabel(detail.kind)}</Badge>
          {detail.archived && <Badge className="bg-muted text-muted-foreground">archived</Badge>}
          {detail.updated && <span className="text-xs text-muted-foreground">last changed {detail.updated}</span>}
        </div>
      </header>
      <div className="grid gap-8 xl:grid-cols-[minmax(0,1fr)_20rem]">
        <div className="min-w-0 space-y-6">
          {children?.(detail)}
          <BodyEditor key={detail.body} item={detail} body={detail.body} items={items} />
        </div>
        <aside className="space-y-5 xl:border-l xl:border-border xl:pl-6">
          <ItemEditors item={detail} />
          <dl className="space-y-1.5 border-t border-border pt-3">
            {detail.waiting_for.length > 0 && (
              <Field label="Waiting for">
                <span className="text-violet-700 dark:text-violet-300">{waitingSummary(detail, peopleOf(nodes, detail.node))}</span>
                {detail.waiting_on && <span className="text-muted-foreground"> — {detail.waiting_on}</span>}
              </Field>
            )}
            {detail.type && <Field label="Type">{detail.type}</Field>}
            {detail.release && <Field label="Release">{detail.release}</Field>}
            {detail.milestone && <Field label="Milestone">{detail.milestone}</Field>}
            <Field label="Created">{detail.created}</Field>
            {detail.external.length > 0 && (
              <Field label="External">
                <Externals detail={detail} nodes={nodes} />
              </Field>
            )}
            {refs.length > 0 && <Field label="Refs">{refs.join(', ')}</Field>}
            {custom.map(([key, value]) => (
              <Field key={key} label={key}>
                {value}
              </Field>
            ))}
            <Field label="Path">
              <code className="font-mono text-xs">{detail.path}</code>
            </Field>
          </dl>
          <CodeLinks title="Branches" icon={GitBranch} links={detail.branches.map(branchLink)} />
          <CodeLinks title="Pull requests" icon={GitPullRequest} links={detail.pull_requests.map(pullRequestLink)} />
          <RelatedList title="Parent" items={relations.parent ? [relations.parent] : []} scope={detail.node} />
          <RelatedList title="Children" items={relations.children} scope={detail.node} />
          <RelatedList title="Depends on" items={relations.depends_on} scope={detail.node} />
          <RelatedList title="Needed by" items={relations.dependents} scope={detail.node} />
          <Unresolved references={relations.unresolved} />
        </aside>
      </div>
    </article>
  )
}
