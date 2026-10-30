import { CircleCheck, CircleX, Copy, ExternalLink, Lock, Pencil, Plus, RefreshCw, Trash2, TriangleAlert } from 'lucide-react'
import { useState } from 'react'
import { brokenChecks, routeOwner, useProxy } from '@/entities/proxy'
import type { ProxyRoute } from '@/shared/api'
import { cn } from '@/shared/lib'
import { Button, ConfirmDialog, DataTable, ErrorState, LoadingState, PageHeader, toast, type Column } from '@/shared/ui'
import { useRemoveRoute } from '../api/use-route-edits'
import { Fold } from './fold'
import { RouteDialog } from './route-dialog'

const commands = [
  { command: 'ariane proxy setup', does: 'Aliases 127.0.0.2 and fd61:7269:616e:6500::2, redirects their port 80 to the front door and declares every name in /etc/hosts under both.' },
  { command: 'ariane proxy teardown', does: 'Undoes all three.' },
]

function Verdict({ ok, label }: { ok: boolean; label: string }) {
  const Icon = ok ? CircleCheck : CircleX
  return (
    <span className={cn('inline-flex items-center gap-1 text-xs font-medium', ok ? 'text-emerald-700 dark:text-emerald-300' : 'text-red-700 dark:text-red-300')}>
      <Icon className="size-3.5 shrink-0" aria-hidden />
      {label}
    </span>
  )
}

function CopyCommand({ command }: { command: string }) {
  return (
    <div className="flex items-center gap-2">
      <code className="rounded bg-muted px-2 py-0.5 font-mono text-xs">{command}</code>
      <button
        type="button"
        aria-label={`Copy ${command}`}
        className="icon-button size-5.5"
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
          className="inline-flex items-center gap-1 font-mono text-xs text-accent hover:underline"
          aria-label={`Open http://${route.host} in a new tab`}
        >
          {route.host}
          <ExternalLink className="size-3" aria-hidden />
        </a>
      ),
    },
    { id: 'port', header: 'Port', sortValue: (route) => route.port, className: 'font-mono text-xs tabular-nums', cell: (route) => route.port },
    {
      id: 'owner',
      header: 'Owner',
      sortValue: (route) => routeOwner(route),
      cell: (route) => <span className={cn('text-xs', route.node === null ? 'text-muted-foreground' : 'font-mono')}>{routeOwner(route)}</span>,
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
      headerClassName: 'w-20',
      cell: (route) =>
        route.node === null ? (
          <span className="flex justify-end" title="From config.toml: ariane config set server.host <name>">
            <Lock className="size-3.5 text-muted-foreground" aria-label="Read-only: from config.toml" />
          </span>
        ) : (
          <div className="flex justify-end gap-1">
            <button type="button" className="icon-button size-7" aria-label={`Edit ${route.host}`} onClick={() => setEditing({ route })}>
              <Pencil className="size-3.5" aria-hidden />
            </button>
            <button type="button" className="icon-button size-7 hover:text-red-600" aria-label={`Remove ${route.host}`} onClick={() => setRemoving(route)}>
              <Trash2 className="size-3.5" aria-hidden />
            </button>
          </div>
        ),
    },
  ]

  return (
    <div className="flex flex-col gap-6 p-6">
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
              <RefreshCw className={cn('size-3.5', isFetching && 'animate-spin')} aria-hidden />
              {isFetching ? 'Checking…' : 'Re-check'}
            </Button>
            <Button size="sm" onClick={() => setEditing({ route: null })} disabled={!report}>
              <Plus className="size-3.5" aria-hidden />
              Add a route
            </Button>
          </>
        }
      />
      {reminder !== null && (
        <div role="status" className="flex flex-wrap items-center gap-2 rounded-md border border-amber-500/40 bg-amber-500/10 p-3 text-sm">
          <TriangleAlert className="size-4 shrink-0 text-amber-700 dark:text-amber-300" aria-hidden />
          <span>
            <span className="font-mono">{reminder}</span> is not in /etc/hosts yet: run <code>ariane proxy setup</code> again to declare it.
          </span>
          <button type="button" className="ml-auto text-xs text-muted-foreground hover:text-foreground" onClick={() => setReminder(null)}>
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
          <section className="space-y-2" aria-labelledby="proxy-routes">
            <h2 id="proxy-routes" className="text-sm font-semibold">
              Routes <span className="font-normal text-muted-foreground">({report.routes.length})</span>
            </h2>
            <DataTable rows={report.routes} columns={routeColumns} rowKey={(route) => route.host} storageKey="proxy-routes" label="Routes" />
            <p className="text-xs text-muted-foreground">
              The interface's own route comes from <code>server.host</code> in config.toml and is changed with <code>ariane config set server.host &lt;name&gt;</code>.
              The others are the <code>[proxy]</code> table of each node's node.toml.
            </p>
            {report.problems.length > 0 && (
              <div className="space-y-1 rounded-md border border-amber-500/40 bg-amber-500/10 p-3 text-sm">
                <p className="font-medium">Left out</p>
                <ul className="list-disc space-y-0.5 pl-4 text-xs">
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
            <p className="text-xs text-muted-foreground">The checks of `ariane proxy status`, from the machine inward to each route.</p>
            <ul className="divide-y divide-border rounded-lg border border-border">
              {report.checks.map((check, index) => (
                <li key={`${check.what}-${index}`} className="grid gap-1 p-3 md:grid-cols-[5.5rem_minmax(0,1fr)]">
                  <Verdict ok={check.ok} label={check.ok ? 'ok' : 'broken'} />
                  <div className="min-w-0 space-y-0.5 text-sm">
                    <p className="font-mono text-xs">{check.what}</p>
                    {!check.ok && check.hint !== '' && <p className="text-xs text-muted-foreground">{check.hint}</p>}
                  </div>
                </li>
              ))}
            </ul>
          </Fold>

          <Fold
            title="The root step"
            openWhen={needsRoot}
            summary={<span className="text-xs text-muted-foreground">{needsRoot ? 'Run ariane proxy setup' : 'Nothing to run'}</span>}
          >
            <p className="text-xs text-muted-foreground">
              These commands need root: they run through sudo, which asks for your password, and what they set lasts until the next reboot. The interface never
              runs them; copy one into a terminal.
            </p>
            <ul className="divide-y divide-border rounded-lg border border-border">
              {commands.map((entry) => (
                <li key={entry.command} className="grid gap-2 p-3 md:grid-cols-[16rem_minmax(0,1fr)]">
                  <CopyCommand command={entry.command} />
                  <p className="text-xs text-muted-foreground">{entry.does}</p>
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
