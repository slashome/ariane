type Point = [number, number]

export type SpoolLayer = { d: string; shade?: { from: Point; to: Point } }

export const spoolViewBox = '0 0 24 24'
export const spoolStrokeWidth = 3.25
export const spoolShadeOpacity = 0.55

export const spoolLayers: SpoolLayer[] = [
  { d: 'M17 13.44 4.442 17.082' },
  { d: 'M17 13.44 12.678 14.693', shade: { from: [17, 13.44], to: [12.678, 14.693] } },
  { d: 'M4.442 17.082A2 2 0 0 0 4.982 21H19a2 2 0 0 0 .558-3.921l-1.115-.32A2 2 0 0 1 17 14.837V7.66' },
  { d: 'M17 7.66V12.2', shade: { from: [17, 7.66], to: [17, 12.2] } },
  { d: 'M7 10.56 19.558 6.918' },
  { d: 'M7 10.56 11.322 9.307', shade: { from: [7, 10.56], to: [11.322, 9.307] } },
  { d: 'M19.558 6.918A2 2 0 0 0 19.018 3H5a2 2 0 0 0-.558 3.921l1.115.32A2 2 0 0 1 7 9.163v7.178' },
  { d: 'M7 16.341V11.8', shade: { from: [7, 16.341], to: [7, 11.8] } },
  { d: 'M9.5 15.615 4.442 17.082' },
]

export function spoolSvg(color: string): string {
  const gradients = spoolLayers
    .map((layer, index) =>
      layer.shade
        ? `<linearGradient id="s${index}" gradientUnits="userSpaceOnUse" x1="${layer.shade.from[0]}" y1="${layer.shade.from[1]}" x2="${layer.shade.to[0]}" y2="${layer.shade.to[1]}"><stop stop-color="#000" stop-opacity="${spoolShadeOpacity}"/><stop offset="1" stop-color="#000" stop-opacity="0"/></linearGradient>`
        : '',
    )
    .join('')
  const strokes = spoolLayers
    .map((layer, index) =>
      layer.shade ? `<path d="${layer.d}" stroke="url(#s${index})" stroke-linecap="butt"/>` : `<path d="${layer.d}"/>`,
    )
    .join('')
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${spoolViewBox}" fill="none" stroke="${color}" stroke-width="${spoolStrokeWidth}" stroke-linecap="round" stroke-linejoin="round"><defs>${gradients}</defs>${strokes}</svg>`
}
