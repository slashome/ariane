import { Pencil, Save, X } from 'lucide-react'
import { useState } from 'react'
import { useUpdateItem } from '@/entities/item'
import type { Item } from '@/shared/api'
import { Button, Markdown } from '@/shared/ui'
import { ItemBody } from './item-body'

// The body of an item, read, or edited as Markdown with its rendering beside it. Saving sends the text
// last read with the new one: a file changed meanwhile is refused, never merged (ADR-0012).
export function BodyEditor({ item, body, items }: { item: Item; body: string; items: Item[] }) {
  const update = useUpdateItem()
  const [draft, setDraft] = useState<string | null>(null)

  if (draft === null) {
    return (
      <div className="group relative">
        <Button size="sm" variant="outline" className="float-right ml-3" onClick={() => setDraft(body)}>
          <Pencil className="size-3.5" aria-hidden />
          Edit
        </Button>
        <ItemBody item={item} body={body} items={items} />
      </div>
    )
  }

  const save = () =>
    update.mutate({ item, changes: { body: { from: body, to: draft } }, optimistic: {} }, { onSuccess: () => setDraft(null) })

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between gap-2">
        <p className="text-xs text-muted-foreground">Markdown — the rendering is on the right. ⌘↵ saves.</p>
        <div className="flex gap-2">
          <Button size="sm" variant="ghost" onClick={() => setDraft(null)}>
            <X className="size-3.5" aria-hidden />
            Cancel
          </Button>
          <Button size="sm" disabled={draft === body || update.isPending} onClick={save}>
            <Save className="size-3.5" aria-hidden />
            {update.isPending ? 'Saving…' : 'Save'}
          </Button>
        </div>
      </div>
      <div className="grid gap-4 lg:grid-cols-2">
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
          className="min-h-[60vh] w-full resize-y rounded-md border border-border bg-background p-3 font-mono text-sm leading-relaxed focus-visible:outline-2 focus-visible:outline-ring"
        />
        <div className="min-h-[60vh] overflow-auto rounded-md border border-dashed border-border p-3">
          <Markdown source={draft} />
        </div>
      </div>
    </div>
  )
}
