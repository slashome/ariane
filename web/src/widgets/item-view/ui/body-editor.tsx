import { useState } from 'react'
import { useUpdateItem } from '@/entities/item'
import type { Item } from '@/shared/api'
import { Button, Icon, Markdown } from '@/shared/ui'
import { ItemBody } from './item-body'
import './item-view.css'

// The body of an item, read, or edited as Markdown with its rendering beside it. Saving sends the text
// last read with the new one: a file changed meanwhile is refused, never merged (ADR-0012).
export function BodyEditor({ item, body, items }: { item: Item; body: string; items: Item[] }) {
  const update = useUpdateItem()
  const [draft, setDraft] = useState<string | null>(null)

  if (draft === null) {
    return (
      <div className="body-editor__read">
        <Button size="sm" variant="outline" className="body-editor__edit" onClick={() => setDraft(body)}>
          <Icon name="edit" className="body-editor__icon" />
          Edit
        </Button>
        <ItemBody item={item} body={body} items={items} />
      </div>
    )
  }

  const save = () =>
    update.mutate({ item, changes: { body: { from: body, to: draft } }, optimistic: {} }, { onSuccess: () => setDraft(null) })

  return (
    <div className="body-editor">
      <div className="body-editor__toolbar">
        <p className="body-editor__hint">Markdown — the rendering is on the right. ⌘↵ saves.</p>
        <div className="body-editor__actions">
          <Button size="sm" variant="ghost" onClick={() => setDraft(null)}>
            <Icon name="close" className="body-editor__icon" />
            Cancel
          </Button>
          <Button size="sm" disabled={draft === body || update.isPending} onClick={save}>
            <Icon name="save" className="body-editor__icon" />
            {update.isPending ? 'Saving…' : 'Save'}
          </Button>
        </div>
      </div>
      <div className="body-editor__panes">
        <textarea
          aria-label="Body"
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
              event.preventDefault()
              save()
            }
          }}
          spellCheck
          className="body-editor__source"
        />
        <div className="body-editor__preview">
          <Markdown source={draft} />
        </div>
      </div>
    </div>
  )
}
