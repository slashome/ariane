import type { ReactNode } from 'react'

export function ComingSoon({ title, children }: { title: string; children?: ReactNode }) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-2 p-10 text-center">
      <h1 className="text-lg font-semibold">{title}</h1>
      <p className="text-sm text-muted-foreground">{children ?? 'Coming in step 3.'}</p>
    </div>
  )
}
