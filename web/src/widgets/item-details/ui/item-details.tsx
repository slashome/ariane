import type { ReactNode } from 'react'
import { ItemLink, waitingSummary } from '@/entities/item'
import { ItemEditors } from '@/features/edit-item'
import { SyncItemButton } from '@/features/sync-item'
import { externalUrl, peopleOf, trackerOf, useNodes } from '@/entities/node'
import { kindLabel } from '@/entities/schema'
import type { Item, Tracker } from '@/shared/api'
import { displayNode } from '@/shared/lib'
import { Icon, Sheet } from '@/shared/ui'
import './item-details.css'

function Field({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div className="item-details__field">
      <dt className="item-details__label">{label}</dt>
      <dd className="item-details__value">{value ?? <span className="item-details__none">—</span>}</dd>
    </div>
  )
}

function ExternalLinks({ entries, tracker }: { entries: string[]; tracker: Tracker | null }) {
  return (
    <ul className="item-details__externals">
      {entries.map((entry) => {
        const url = externalUrl(tracker, entry)
        return (
          <li key={entry} className="item-details__external">
            {url ? (
              <a href={url} target="_blank" rel="noreferrer" className="item-details__external-link">
                {entry}
                <Icon name="external-link" className="item-details__external-icon" />
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

// Where the previewed item stands in the list it was opened from, to step through it.
export interface Stepper {
  position: number
  count: number
  onPrevious?: () => void
  onNext?: () => void
}

export function ItemDetails({ item, onClose, stepper }: { item: Item | null; onClose: () => void; stepper?: Stepper }) {
  const { data: nodes = [] } = useNodes()
  return (
    <Sheet
      open={item !== null}
      onOpenChange={(open) => !open && onClose()}
      title={item?.title ?? item?.path ?? ''}
      description={item ? `${displayNode(item.node)} · ${item.id ?? 'no id'}` : undefined}
    >
      {item && item.id !== null && (
        <ItemLink item={item} className="item-details__open">
          <Icon name="maximize" className="item-details__icon" />
          Open the page: body, parent, children, dependencies
        </ItemLink>
      )}
      {item && <ItemEditors item={item} />}
      {item && !item.archived && (
        <div>
          <SyncItemButton item={item} />
        </div>
      )}
      {item && (
        <dl className="item-details__fields">
          <Field label="Kind" value={kindLabel(item.kind)} />
          <Field label="Archived" value={item.archived ? 'yes' : 'no'} />
          {item.waiting_for.length > 0 && (
            <Field
              label="Waiting for"
              value={`${waitingSummary(item, peopleOf(nodes, item.node))}${item.waiting_on ? ` — ${item.waiting_on}` : ''}`}
            />
          )}
          <Field label="Created" value={item.created} />
          <Field label="External" value={
              item.external.length > 0 ? (
                <ExternalLinks entries={item.external} tracker={trackerOf(nodes, item.node)} />
              ) : null
            }
          />
          <Field label="Schema" value={item.version === null ? 'not versioned' : `version ${item.version}`} />
          <Field label="Path" value={<code className="item-details__path">{item.path}</code>} />
        </dl>
      )}
      {item && stepper && (
        <nav aria-label="Step through the items" className="item-details__stepper">
          <button type="button" className="lu-icon-button item-details__step" aria-label="Previous item" disabled={!stepper.onPrevious} onClick={stepper.onPrevious}>
            <Icon name="chevron-left" className="item-details__step-icon" />
          </button>
          <span className="item-details__position">
            {stepper.position} / {stepper.count}
          </span>
          <button type="button" className="lu-icon-button item-details__step" aria-label="Next item" disabled={!stepper.onNext} onClick={stepper.onNext}>
            <Icon name="chevron-right" className="item-details__step-icon" />
          </button>
        </nav>
      )}
    </Sheet>
  )
}
