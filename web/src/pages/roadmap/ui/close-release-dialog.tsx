import { Copy } from 'lucide-react'
import { useState, type ReactNode } from 'react'
import type { ClosingWork, ReleaseClosing } from '@/shared/api'
import { Button, Dialog, ErrorState, LoadingState, Switch } from '@/shared/ui'
import { useClosePreview, type CloseTarget } from '../api/use-release-close'

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-1">
      <h3 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">{title}</h3>
      {children}
    </section>
  )
}

function WorkLine({ work, note }: { work: ClosingWork; note?: string }) {
  return (
    <li className="flex items-baseline gap-2 text-sm">
      <span className="shrink-0 font-mono text-[11px] text-muted-foreground">{work.id}</span>
      <span className="truncate">{work.title}</span>
      {note && <span className="shrink-0 text-xs text-muted-foreground">{note}</span>}
    </li>
  )
}

function OpenWork({ closing, carryOver, onCarryOver }: { closing: ReleaseClosing; carryOver: boolean; onCarryOver: (next: boolean) => void }) {
  if (closing.open.length === 0) {
    return null
  }
  return (
    <Section title={`Open work (${closing.open.length})`}>
      <p className="text-sm text-muted-foreground">
        {carryOver && closing.carry_to
          ? `It moves to ${closing.carry_to}, written on each item.`
          : 'The release cannot close while this work is open.'}
      </p>
      <ul className="flex max-h-40 flex-col gap-0.5 overflow-y-auto">
        {closing.open.map((work) => (
          <WorkLine key={work.reference} work={work} note={work.status ?? undefined} />
        ))}
      </ul>
      <label className="flex items-center gap-2 text-sm">
        <Switch checked={carryOver} disabled={closing.carry_to === null} onCheckedChange={onCarryOver} aria-label="Carry over the open work" />
        {closing.carry_to === null ? 'No later release to carry it over to' : `Carry it over to ${closing.carry_to}`}
      </label>
    </Section>
  )
}

function Plan({ closing, carryOver, onCarryOver }: { closing: ReleaseClosing; carryOver: boolean; onCarryOver: (next: boolean) => void }) {
  const [copied, setCopied] = useState(false)
  const copy = () => {
    void navigator.clipboard?.writeText(closing.changelog.markdown).then(() => setCopied(true))
  }
  return (
    <div className="flex min-h-0 flex-col gap-4 overflow-y-auto">
      <OpenWork closing={closing} carryOver={carryOver} onCarryOver={onCarryOver} />
      <Section title={`Archived (${closing.archived.length})`}>
        {closing.archived.length === 0 ? (
          <p className="text-sm text-muted-foreground">Nothing to archive.</p>
        ) : (
          <ul className="flex max-h-40 flex-col gap-0.5 overflow-y-auto">
            {closing.archived.map((archived) => (
              <WorkLine
                key={archived.reference}
                work={archived}
                note={archived.references > 0 ? `${archived.references} links rewritten` : undefined}
              />
            ))}
          </ul>
        )}
      </Section>
      {closing.kept.length > 0 && (
        <Section title={`Kept outside archives (${closing.kept.length})`}>
          <ul className="flex max-h-32 flex-col gap-0.5 overflow-y-auto">
            {closing.kept.map((kept) => (
              <WorkLine key={kept.reference} work={kept} note={kept.reason} />
            ))}
          </ul>
        </Section>
      )}
      <Section title="Changelog entry">
        <div className="relative">
          <pre aria-label="Changelog entry" className="max-h-60 overflow-auto rounded-md border border-border bg-muted/40 p-3 font-mono text-xs whitespace-pre-wrap">
            {closing.changelog.markdown}
          </pre>
          <Button variant="outline" size="sm" className="absolute top-2 right-2" onClick={copy}>
            <Copy className="size-3.5" aria-hidden />
            {copied ? 'Copied' : 'Copy'}
          </Button>
        </div>
      </Section>
    </div>
  )
}

export function CloseReleaseDialog({
  target,
  pending,
  onClose,
  onConfirm,
}: {
  target: CloseTarget | null
  pending: boolean
  onClose: () => void
  onConfirm: (carryOver: boolean) => void
}) {
  const [carryOver, setCarryOver] = useState(false)
  const preview = useClosePreview(target, carryOver)
  const closing = preview.data
  const blocked = closing === undefined || closing.blocked
  return (
    <Dialog
      open={target !== null}
      onOpenChange={(next) => !next && onClose()}
      title={`Close ${target?.name ?? ''}`}
      description="Its finished work moves under archives/, links rewritten, and the release is marked closed in node.toml. Nothing is committed."
      className="w-[40rem]"
    >
      {preview.isPending ? (
        <LoadingState rows={3} label="Computing the plan" />
      ) : preview.isError ? (
        <ErrorState message={preview.error.message} />
      ) : closing ? (
        <Plan closing={closing} carryOver={carryOver} onCarryOver={setCarryOver} />
      ) : null}
      <div className="flex justify-end gap-2">
        <Button variant="outline" size="sm" onClick={onClose}>
          Cancel
        </Button>
        <Button size="sm" disabled={pending || blocked || preview.isFetching} onClick={() => onConfirm(carryOver)}>
          Close release
        </Button>
      </div>
    </Dialog>
  )
}
