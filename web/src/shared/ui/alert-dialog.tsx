import { AlertDialog } from 'radix-ui'
import type { ReactNode } from 'react'
import { buttonVariants } from './button'

interface ConfirmDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  title: ReactNode
  description: ReactNode
  confirmLabel: string
  pending?: boolean
  onConfirm: () => void
}

// The action button does not close the dialog by itself: the caller closes it once the work is done.
export function ConfirmDialog({ open, onOpenChange, title, description, confirmLabel, pending, onConfirm }: ConfirmDialogProps) {
  return (
    <AlertDialog.Root open={open} onOpenChange={onOpenChange}>
      <AlertDialog.Portal>
        <AlertDialog.Overlay className="fixed inset-0 z-50 bg-black/30" />
        <AlertDialog.Content className="fixed top-1/2 left-1/2 z-50 flex w-[28rem] max-w-[calc(100%-2rem)] -translate-x-1/2 -translate-y-1/2 flex-col gap-3 rounded-lg border border-border bg-background p-5 shadow-xl focus:outline-none">
          <AlertDialog.Title className="text-base font-semibold">{title}</AlertDialog.Title>
          <AlertDialog.Description className="text-sm break-words text-muted-foreground">{description}</AlertDialog.Description>
          <div className="mt-2 flex justify-end gap-2">
            <AlertDialog.Cancel className={buttonVariants({ variant: 'outline', size: 'sm' })}>Cancel</AlertDialog.Cancel>
            <AlertDialog.Action
              className={buttonVariants({ variant: 'destructive', size: 'sm' })}
              disabled={pending}
              onClick={(event) => {
                event.preventDefault()
                onConfirm()
              }}
            >
              {confirmLabel}
            </AlertDialog.Action>
          </div>
        </AlertDialog.Content>
      </AlertDialog.Portal>
    </AlertDialog.Root>
  )
}
