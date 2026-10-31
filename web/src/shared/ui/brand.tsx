import { SpoolLogo } from './spool-logo'
import './brand.css'

export function Brand() {
  return (
    <span className="brand">
      <SpoolLogo className="brand__logo" />
      <span className="brand__name">ariane</span>
    </span>
  )
}
