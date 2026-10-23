import { spoolSvg } from '@/shared/ui'

export function faviconSvg(color: string): string {
  return spoolSvg(color)
}

export function faviconHref(color: string): string {
  return `data:image/svg+xml,${encodeURIComponent(faviconSvg(color))}`
}
