import type { ReactNode } from 'react'
import { Link, useLocation } from 'react-router'
import { StatusBadge, useItem, useItems, waitingSummary } from '@/entities/item'
import { externalUrl, peopleOf, statusLabel, trackerOf, useNodes } from '@/entities/node'
import { kindLabel } from '@/entities/schema'
import type { ItemDetail, NodeSummary } from '@/shared/api'
import { scopeSearch, sectionOf } from '@/shared/config'
import { displayNode } from '@/shared/lib'
import { Badge, EmptyState, ErrorState, Icon, LoadingState } from '@/shared/ui'
import { branchLink, pullRequestLink, type CodeLink } from '../model/code-links'
import { customEntries, refsOf } from '../model/header-keys'
import { BodyEditor } from './body-editor'
import { ItemEditors } from '@/features/edit-item'
import { RelatedList, Unresolved } from './related-list'
import './item-view.css'

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="item-view__field">
      <dt className="item-view__label">{label}</dt>
      <dd className="item-view__value">{children ?? <span className="item-view__none">—</span>}</dd>
    </div>
  )
}

function Externals({ detail, nodes }: { detail: ItemDetail; nodes: NodeSummary[] }) {
  if (detail.external.length === 0) {
    return null
  }
  const tracker = trackerOf(nodes, detail.node)
  return (
    <ul className="item-view__list item-view__list--tight">
      {detail.external.map((entry) => {
        const url = externalUrl(tracker, entry)
        return (
          <li key={entry} className="item-view__external">
            {url ? (
              <a href={url} target="_blank" rel="noreferrer" className="item-view__link item-view__external-link">
                {entry}
                <Icon name="external-link" className="item-view__external-icon" />
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

function CodeLinks({ title, icon, links }: { title: string; icon: string; links: CodeLink[] }) {
  if (links.length === 0) {
    return null
  }
  return (
    <section className="item-view__panel">
      <h3 className="item-view__panel-title">
        {title} <span className="item-view__panel-count">({links.length})</span>
      </h3>
      <ul className="item-view__list">
        {links.map((link) => (
          <li key={link.label} className="item-view__code-link">
            <Icon name={icon} className="item-view__code-icon" />
            {link.url ? (
              <a href={link.url} target="_blank" rel="noreferrer" className="item-view__code-label item-view__link">
                {link.label}
              </a>
            ) : (
              <span className="item-view__code-label">{link.label}</span>
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
    <article className="item-view">
      <nav aria-label="Breadcrumb" className="item-view__breadcrumb">
        <Link to={{ pathname: sectionOf(detail.kind), search: scopeSearch(search) }} className="item-view__section-link">
          {sectionLabel}
        </Link>
        <Icon name="chevron-right" className="item-view__crumb-icon" />
        <span className="item-view__mono">{displayNode(detail.node)}</span>
        <Icon name="chevron-right" className="item-view__crumb-icon" />
        <span className="item-view__current">{detail.id}</span>
      </nav>
      <header className="item-view__header">
        <h1 className="item-view__title">{detail.title ?? detail.path}</h1>
        <div className="item-view__badges">
          <StatusBadge status={detail.status} category={detail.category} label={statusLabel(nodes, detail.node, detail.status)} />
          <Badge className="item-view__badge">{kindLabel(detail.kind)}</Badge>
          {detail.archived && <Badge className="item-view__badge">archived</Badge>}
          {detail.updated && <span className="item-view__updated">last changed {detail.updated}</span>}
        </div>
      </header>
      <div className="item-view__layout">
        <div className="item-view__main">
          {children?.(detail)}
          <BodyEditor key={detail.body} item={detail} body={detail.body} items={items} />
        </div>
        <aside className="item-view__aside">
          <ItemEditors item={detail} />
          <dl className="item-view__fields">
            {detail.waiting_for.length > 0 && (
              <Field label="Waiting for">
                <span className="item-view__waiting">{waitingSummary(detail, peopleOf(nodes, detail.node))}</span>
                {detail.waiting_on && <span className="item-view__waiting-on"> — {detail.waiting_on}</span>}
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
              <code className="item-view__path">{detail.path}</code>
            </Field>
          </dl>
          <CodeLinks title="Branches" icon="branch" links={detail.branches.map(branchLink)} />
          <CodeLinks title="Pull requests" icon="pull-request" links={detail.pull_requests.map(pullRequestLink)} />
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
