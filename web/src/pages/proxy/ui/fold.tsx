import { useId, useState, type ReactNode } from 'react'
import { cn } from '@/shared/lib'
import { Icon } from '@/shared/ui'
import './proxy.css'

type Override = { when: boolean; open: boolean }

export function Fold({
  title,
  summary,
  openWhen,
  children,
}: {
  title: string
  summary: ReactNode
  openWhen: boolean
  children: ReactNode
}) {
  const [override, setOverride] = useState<Override | null>(null)
  const open = override !== null && override.when === openWhen ? override.open : openWhen
  const body = useId()
  return (
    <section className="proxy-fold" aria-label={title}>
      <div className="proxy-fold__header">
        <button
          type="button"
          aria-expanded={open}
          aria-controls={body}
          onClick={() => setOverride({ when: openWhen, open: !open })}
          className="proxy-fold__toggle"
        >
          <Icon name="chevron-down" className={cn('proxy-fold__chevron', !open && 'proxy-fold__chevron--folded')} />
          {title}
        </button>
        {summary}
      </div>
      {open && (
        <div id={body} className="proxy-fold__body">
          {children}
        </div>
      )}
    </section>
  )
}
