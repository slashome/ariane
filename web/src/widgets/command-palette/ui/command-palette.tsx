import { Dialog as DialogPrimitive } from 'radix-ui'
import { Icon, Kbd } from '@/shared/ui'
import { usePaletteToggle } from '../model/use-palette'
import { PaletteContent } from './palette-content'
import './command-palette.css'

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
          className="command-palette__trigger"
        >
          <Icon name="command" className="command-palette__icon" />
          Commands
          <Kbd>{shortcut}</Kbd>
        </button>
      </DialogPrimitive.Trigger>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay className="command-palette__overlay" />
        <PaletteContent close={() => setOpen(false)} />
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  )
}
