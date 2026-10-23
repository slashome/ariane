import { useId } from 'react'
import { spoolLayers, spoolShadeOpacity, spoolStrokeWidth, spoolViewBox } from './spool'

export function SpoolLogo({ className }: { className?: string }) {
  const prefix = useId()
  return (
    <svg
      viewBox={spoolViewBox}
      fill="none"
      stroke="currentColor"
      strokeWidth={spoolStrokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden
    >
      <defs>
        {spoolLayers.map((layer, index) =>
          layer.shade ? (
            <linearGradient
              key={index}
              id={`${prefix}${index}`}
              gradientUnits="userSpaceOnUse"
              x1={layer.shade.from[0]}
              y1={layer.shade.from[1]}
              x2={layer.shade.to[0]}
              y2={layer.shade.to[1]}
            >
              <stop stopColor="#000" stopOpacity={spoolShadeOpacity} />
              <stop offset="1" stopColor="#000" stopOpacity="0" />
            </linearGradient>
          ) : null,
        )}
      </defs>
      {spoolLayers.map((layer, index) =>
        layer.shade ? (
          <path key={index} d={layer.d} stroke={`url(#${prefix}${index})`} strokeLinecap="butt" />
        ) : (
          <path key={index} d={layer.d} />
        ),
      )}
    </svg>
  )
}
