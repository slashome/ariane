import { Dialog as DialogPrimitive } from 'radix-ui'
import { Command as CommandIcon } from 'lucide-react'
import { Kbd } from '@/shared/ui'
import { usePaletteToggle } from '../model/use-palette'
import { PaletteContent } from './palette-content'

// The command palette, opened with ⌘K (Ctrl+K outside Apple systems) or its button: items, views,
// nodes, actions and the full text, all from the keyboard.
export function CommandPalette() {
  const { open, setOpen, shortcut } = usePaletteToggle()
  return (
    <DialogPrimitive.Root open={open} onOpenChange={setOpen}>
      <DialogPrimitive.Trigger asChild>
        <button
          type="button"
          aria-keyshortcuts={shortcut === '⌘K' ? 'Meta+K' : 'Control+K'}
          className="inline-flex h-9 items-center gap-2 rounded-md border border-border px-2.5 text-sm text-muted-foreground hover:bg-muted hover:text-foreground"
        >
          <CommandIcon className="size-4" aria-hidden />
          Commands
          <Kbd>{shortcut}</Kbd>
        </button>
      </DialogPrimitive.Trigger>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay className="fixed inset-0 z-50 bg-black/30" />
        <PaletteContent close={() => setOpen(false)} />
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  )
}
