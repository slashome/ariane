import { SpoolLogo } from './spool-logo'

export function Brand() {
  return (
    <span className="flex items-center gap-1.5 text-primary">
      <SpoolLogo className="size-6 shrink-0" />
      <span className="font-brand -translate-y-px text-xl leading-none font-bold">ariane</span>
    </span>
  )
}
