import { useState } from 'react'
import { brokenChecks, routeOwner, useProxy } from '@/entities/proxy'
import type { ProxyRoute } from '@/shared/api'
import { cn } from '@/shared/lib'
import { Button, ConfirmDialog, DataTable, ErrorState, Icon, LoadingState, PageHeader, toast, type Column } from '@/shared/ui'
import { useRemoveRoute } from '../api/use-route-edits'
import { Fold } from './fold'
import { RouteDialog } from './route-dialog'
import './proxy.css'

const commands = [
  { command: 'ariane proxy setup', does: 'Aliases 127.0.0.2 and fd61:7269:616e:6500::2, redirects their port 80 to the front door and declares every name in /etc/hosts under both.' },
  { command: 'ariane proxy teardown', does: 'Undoes all three.' },
]

function Verdict({ ok, label }: { ok: boolean; label: string }) {
  return (
    <span className={cn('proxy-verdict', ok ? 'proxy-verdict--ok' : 'proxy-verdict--broken')}>
      <Icon name={ok ? 'success' : 'error'} className="proxy-verdict__icon" />
      {label}
    </span>
  )
}

function CopyCommand({ command }: { command: string }) {
  return (
    <div className="copy-command">
      <code className="copy-command__code">{command}</code>
      <button
        type="button"
        aria-label={`Copy ${command}`}
        className="lu-icon-button copy-command__button"
        onClick={() => {
          void navigator.clipboard?.writeText(command)
          toast.success('Command copied')
        }}
      >
        <Icon name="copy" className="copy-command__icon" />
      </button>
    </div>
  )
}

type Editing = { route: ProxyRoute | null }

export function ProxyPage() {
  const { data: report, isPending, error, refetch, isFetching } = useProxy()
  const remove = useRemoveRoute()
  const [editing, setEditing] = useState<Editing | null>(null)
  const [removing, setRemoving] = useState<ProxyRoute | null>(null)
  const [reminder, setReminder] = useState<string | null>(null)

  const broken = report ? brokenChecks(report) : 0
  const needsRoot = report ? !report.ok || reminder !== null || report.routes.some((route) => !route.declared) : false

  const routeColumns: Column<ProxyRoute>[] = [
    {
      id: 'host',
      header: 'Host',
      sortValue: (route) => route.host,
      cell: (route) => (
        <a
          href={`http://${route.host}`}
          target="_blank"
          rel="noreferrer"
          className="proxy-table__host"
          aria-label={`Open http://${route.host} in a new tab`}
        >
          {route.host}
          <Icon name="external-link" className="proxy-table__host-icon" />
        </a>
      ),
    },
    { id: 'port', header: 'Port', sortValue: (route) => route.port, className: 'proxy-table__port', cell: (route) => route.port },
    {
      id: 'owner',
      header: 'Owner',
      sortValue: (route) => routeOwner(route),
      cell: (route) => <span className={cn('proxy-table__owner', route.node === null ? 'proxy-table__owner--config' : 'proxy-table__owner--node')}>{routeOwner(route)}</span>,
    },
    {
      id: 'declared',
      header: '/etc/hosts',
      sortValue: (route) => route.declared,
      cell: (route) => <Verdict ok={route.declared} label={route.declared ? 'declared' : 'missing'} />,
    },
    {
      id: 'listening',
      header: 'Upstream',
      sortValue: (route) => route.listening,
      cell: (route) => <Verdict ok={route.listening} label={route.listening ? 'listening' : 'not listening'} />,
    },
    {
      id: 'actions',
      header: '',
      headerClassName: 'proxy-table__actions-header',
      cell: (route) =>
        route.node === null ? (
          <span className="proxy-actions" title="From config.toml: ariane config set server.host <name>">
            <Icon name="lock" className="proxy-actions__lock" label="Read-only: from config.toml" />
          </span>
        ) : (
          <div className="proxy-actions">
            <button type="button" className="lu-icon-button proxy-actions__button" aria-label={`Edit ${route.host}`} onClick={() => setEditing({ route })}>
              <Icon name="edit" className="proxy-actions__icon" />
            </button>
            <button type="button" className="lu-icon-button proxy-actions__button proxy-actions__button--remove" aria-label={`Remove ${route.host}`} onClick={() => setRemoving(route)}>
              <Icon name="delete" className="proxy-actions__icon" />
            </button>
          </div>
        ),
    },
  ]

  return (
    <div className="proxy-page">
      <PageHeader
        title="Proxy"
        description={
          report
            ? `Local names for the interface and the projects' development servers, through the front door on 127.0.0.1:${report.proxy_port} and [::1]:${report.proxy_port}.`
            : "Local names for the interface and the projects' development servers, through the front door."
        }
        actions={
          <>
            <Button variant="outline" size="sm" onClick={() => void refetch()} disabled={isFetching}>
              <Icon name="refresh" className={cn('proxy-page__button-icon', isFetching && 'lu-spin')} />
              {isFetching ? 'Checking…' : 'Re-check'}
            </Button>
            <Button size="sm" onClick={() => setEditing({ route: null })} disabled={!report}>
              <Icon name="add" className="proxy-page__button-icon" />
              Add a route
            </Button>
          </>
        }
      />
      {reminder !== null && (
        <div role="status" className="proxy-notice">
          <Icon name="warning" className="proxy-notice__icon" />
          <span>
            <span className="proxy-notice__host">{reminder}</span> is not in /etc/hosts yet: run <code>ariane proxy setup</code> again to declare it.
          </span>
          <button type="button" className="proxy-notice__dismiss" onClick={() => setReminder(null)}>
            Dismiss
          </button>
        </div>
      )}
      {error && !report ? (
        <ErrorState message={`Could not check the proxy: ${error.message}`} />
      ) : isPending || !report ? (
        <LoadingState rows={4} label="Checking the proxy…" />
      ) : (
        <>
          <section className="proxy-section" aria-labelledby="proxy-routes">
            <h2 id="proxy-routes" className="proxy-section__title">
              Routes <span className="proxy-section__count">({report.routes.length})</span>
            </h2>
            <DataTable rows={report.routes} columns={routeColumns} rowKey={(route) => route.host} storageKey="proxy-routes" label="Routes" />
            <p className="proxy-page__hint">
              The interface's own route comes from <code>server.host</code> in config.toml and is changed with <code>ariane config set server.host &lt;name&gt;</code>.
              The others are the <code>[proxy]</code> table of each node's node.toml.
            </p>
            {report.problems.length > 0 && (
              <div className="proxy-notice proxy-notice--list">
                <p className="proxy-notice__title">Left out</p>
                <ul className="proxy-notice__list">
                  {report.problems.map((problem) => (
                    <li key={problem}>{problem}</li>
                  ))}
                </ul>
              </div>
            )}
          </section>

          <Fold
            title="The chain"
            openWhen={!report.ok}
            summary={
              <Verdict
                ok={report.ok}
                label={report.ok ? `Every link holds · ${report.checks.length} checks` : `${broken} ${broken === 1 ? 'link' : 'links'} broken`}
              />
            }
          >
            <p className="proxy-page__hint">The checks of `ariane proxy status`, from the machine inward to each route.</p>
            <ul className="proxy-list">
              {report.checks.map((check, index) => (
                <li key={`${check.what}-${index}`} className="proxy-list__item">
                  <Verdict ok={check.ok} label={check.ok ? 'ok' : 'broken'} />
                  <div className="proxy-check">
                    <p className="proxy-check__what">{check.what}</p>
                    {!check.ok && check.hint !== '' && <p className="proxy-page__hint">{check.hint}</p>}
                  </div>
                </li>
              ))}
            </ul>
          </Fold>

          <Fold
            title="The root step"
            openWhen={needsRoot}
            summary={<span className="proxy-page__hint">{needsRoot ? 'Run ariane proxy setup' : 'Nothing to run'}</span>}
          >
            <p className="proxy-page__hint">
              These commands need root: they run through sudo, which asks for your password, and what they set lasts until the next reboot. The interface never
              runs them; copy one into a terminal.
            </p>
            <ul className="proxy-list">
              {commands.map((entry) => (
                <li key={entry.command} className="proxy-list__item proxy-list__item--command">
                  <CopyCommand command={entry.command} />
                  <p className="proxy-page__hint">{entry.does}</p>
                </li>
              ))}
            </ul>
          </Fold>
        </>
      )}
      {editing && report && (
        <RouteDialog
          key={editing.route?.host ?? 'new'}
          route={editing.route}
          report={report}
          onClose={() => setEditing(null)}
          onSaved={(host, setup) => setReminder(setup ? host : null)}
        />
      )}
      <ConfirmDialog
        open={removing !== null}
        onOpenChange={(open) => !open && setRemoving(null)}
        title={`Remove ${removing?.host ?? ''}?`}
        description="Its [proxy] table leaves the node's node.toml. The name stays in /etc/hosts until the next ariane proxy setup."
        confirmLabel={remove.isPending ? 'Removing…' : 'Remove'}
        pending={remove.isPending}
        onConfirm={() => {
          if (removing?.node == null) {
            return
          }
          remove.mutate(removing.node, { onSettled: () => setRemoving(null) })
        }}
      />
    </div>
  )
}
