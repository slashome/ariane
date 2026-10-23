import { SpoolLogo } from './spool-logo'

export function Brand() {
  return (
    <span className="flex items-center gap-2">
      <SpoolLogo className="size-6 shrink-0 text-primary" />
      <span className="font-brand text-lg leading-none font-normal">ariane</span>
    </span>
  )
}
