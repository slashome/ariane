import { Toaster as Sonner, toast } from 'sonner'

export { toast }

export function Toaster() {
  return <Sonner theme="system" position="bottom-right" richColors closeButton />
}
