// The spool of Lucide (ISC), Ariane's thread, drawn in the theme's primary color.
const spool = [
  'M17 13.44 4.442 17.082A2 2 0 0 0 4.982 21H19a2 2 0 0 0 .558-3.921l-1.115-.32A2 2 0 0 1 17 14.837V7.66',
  'm7 10.56 12.558-3.642A2 2 0 0 0 19.018 3H5a2 2 0 0 0-.558 3.921l1.115.32A2 2 0 0 1 7 9.163v7.178',
]

export function faviconSvg(color: string): string {
  const paths = spool.map((d) => `<path d="${d}"/>`).join('')
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="${color}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">${paths}</svg>`
}

export function faviconHref(color: string): string {
  return `data:image/svg+xml,${encodeURIComponent(faviconSvg(color))}`
}
