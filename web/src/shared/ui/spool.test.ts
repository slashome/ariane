import { describe, expect, it } from 'vitest'
import { spoolLayers, spoolSvg } from './spool'

describe('spoolSvg', () => {
  it('draws the strands in the given color', () => {
    expect(spoolSvg('#123456')).toContain('stroke="#123456"')
  })

  it('shades the strand under each of the four crossings', () => {
    const svg = spoolSvg('#123456')
    expect(spoolLayers.filter((layer) => layer.shade)).toHaveLength(4)
    expect(svg.match(/<linearGradient /g)).toHaveLength(4)
    expect(svg.match(/stroke="url\(#s\d+\)"/g)).toHaveLength(4)
  })
})
