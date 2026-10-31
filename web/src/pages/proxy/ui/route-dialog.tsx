import { useState } from 'react'
import { useNodes } from '@/entities/node'
import type { ProxyReport, ProxyRoute } from '@/shared/api'
import { displayNode } from '@/shared/lib'
import { Button, Dialog, Input, NativeSelect } from '@/shared/ui'
import { useSetRoute } from '../api/use-route-edits'
import { needsSetup, parsePort, routeProblem, suggestHost } from '../model/route-form'
import './proxy.css'

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
      className="route-dialog"
    >
      <form
        className="proxy-form"
        onSubmit={(event) => {
          event.preventDefault()
          submit()
        }}
      >
        <label className="proxy-form__field">
          <span className="proxy-form__label">Node</span>
          {route === null ? (
            <NativeSelect className="proxy-form__select" value={node ?? ''} onChange={(event) => setNode(event.target.value === '' ? null : event.target.value)}>
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
            <span className="proxy-form__value">{displayNode(route.node ?? '')}</span>
          )}
        </label>
        <label className="proxy-form__field">
          <span className="proxy-form__label">Host</span>
          <Input
            value={shownHost}
            className="proxy-form__mono"
            placeholder="app.local"
            onChange={(event) => {
              setHostTouched(true)
              setHost(event.target.value)
            }}
          />
          <span className="proxy-form__hint">The name typed in the browser, as in http://{shownHost || 'app.local'}.</span>
        </label>
        <label className="proxy-form__field">
          <span className="proxy-form__label">Port</span>
          <Input value={port} inputMode="numeric" className="proxy-form__mono" placeholder="3000" onChange={(event) => setPort(event.target.value)} />
          <span className="proxy-form__hint">Where the development server listens, on 127.0.0.1.</span>
        </label>
        {touched && problem !== null && (
          <p role="alert" className="proxy-form__problem">
            {problem}
          </p>
        )}
        <div className="proxy-form__actions">
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
