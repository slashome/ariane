export const defaultColor = '#3760c9'

export function isColor(text: string): boolean {
  return /^#[0-9a-f]{6}$/i.test(text)
}

// WCAG relative luminance of a `#rrggbb` color.
export function luminance(color: string): number {
  const channels = [1, 3, 5].map((start) => parseInt(color.slice(start, start + 2), 16) / 255)
  const [red = 0, green = 0, blue = 0] = channels.map((channel) =>
    channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4,
  )
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue
}

// WCAG contrast ratio of two colors, from 1 to 21.
export function contrast(left: string, right: string): number {
  const [light, dark] = [luminance(left), luminance(right)].sort((a, b) => b - a) as [number, number]
  return (light + 0.05) / (dark + 0.05)
}

// Whether a primary color stands out from both page backgrounds enough for links and focus rings
// (3:1, the WCAG minimum for interface components). Dark mode lightens a dark primary, so only
// the light background is checked against the color as chosen.
export function readableOnLight(color: string): boolean {
  return contrast(color, '#fcfcfc') >= 3
}
