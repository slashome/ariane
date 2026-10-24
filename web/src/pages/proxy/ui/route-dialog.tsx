import { useState } from 'react'
import { useNodes } from '@/entities/node'
import type { ProxyReport, ProxyRoute } from '@/shared/api'
import { displayNode } from '@/shared/lib'
import { Button, Dialog, Input, NativeSelect } from '@/shared/ui'
import { useSetRoute } from '../api/use-route-edits'
import { needsSetup, parsePort, routeProblem, suggestHost } from '../model/route-form'

export function RouteDialog({
  route,
  report,
  onClose,
  onSaved,
}: {
  route: ProxyRoute | null
  report: ProxyReport
  onClose: () => void
  onSaved: (host: string, setup: boolean) => void
}) {
  const save = useSetRoute()
  const { data: nodes = [] } = useNodes()
  const routed = new Set(report.routes.map((entry) => entry.node))
  const free = nodes.filter((node) => !routed.has(node.path))
  const [node, setNode] = useState<string | null>(route?.node ?? null)
  const [host, setHost] = useState(route?.host ?? '')
  const [hostTouched, setHostTouched] = useState(route !== null)
  const [port, setPort] = useState(route?.port.toString() ?? '')
  const shownHost = hostTouched || node === null ? host : suggestHost(node)
  const draft = { node, host: shownHost, port }
  const problem = routeProblem(draft, report)
  const touched = node !== null && (shownHost !== '' || port !== '')

  const submit = () => {
    const parsed = parsePort(port)
    if (problem !== null || node === null || parsed === null) {
      return
    }
    const name = shownHost.trim()
    const setup = needsSetup(name, report)
    save.mutate(
      { node, host: name, port: parsed },
      {
        onSuccess: () => {
          onSaved(name, setup)
          onClose()
        },
      },
    )
  }

  return (
    <Dialog
      open
      onOpenChange={(open) => !open && onClose()}
      title={route === null ? 'Add a route' : `Edit ${route.host}`}
      description={
        <>
          Written to the <code>[proxy]</code> table of the node's <code>node.toml</code>, comments and other tables kept. The content repository
          then has a change to commit.
        </>
      }
      className="w-[32rem]"
    >
      <form
        className="flex flex-col gap-4"
        onSubmit={(event) => {
          event.preventDefault()
          submit()
        }}
      >
        <label className="space-y-1 text-sm">
          <span className="font-medium">Node</span>
          {route === null ? (
            <NativeSelect className="w-full font-mono" value={node ?? ''} onChange={(event) => setNode(event.target.value === '' ? null : event.target.value)}>
              <option value="" disabled>
                Choose a node…
              </option>
              {free.map((entry) => (
                <option key={entry.path} value={entry.path}>
                  {displayNode(entry.path)}
                </option>
              ))}
            </NativeSelect>
          ) : (
            <span className="block font-mono text-sm">{displayNode(route.node ?? '')}</span>
          )}
        </label>
        <label className="space-y-1 text-sm">
          <span className="font-medium">Host</span>
          <Input
            value={shownHost}
            className="font-mono"
            placeholder="app.local"
            onChange={(event) => {
              setHostTouched(true)
              setHost(event.target.value)
            }}
          />
          <span className="block text-xs text-muted-foreground">The name typed in the browser, as in http://{shownHost || 'app.local'}.</span>
        </label>
        <label className="space-y-1 text-sm">
          <span className="font-medium">Port</span>
          <Input value={port} inputMode="numeric" className="font-mono" placeholder="3000" onChange={(event) => setPort(event.target.value)} />
          <span className="block text-xs text-muted-foreground">Where the development server listens, on 127.0.0.1.</span>
        </label>
        {touched && problem !== null && (
          <p role="alert" className="text-xs text-red-700 dark:text-red-300">
            {problem}
          </p>
        )}
        <div className="flex justify-end gap-2">
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button type="submit" disabled={problem !== null || save.isPending}>
            {route === null ? 'Add' : 'Save'}
          </Button>
        </div>
      </form>
    </Dialog>
  )
}
