import { Info } from 'lucide-react'
import { Link, useLocation } from 'react-router'
import { searchParamKeys } from '@/shared/config'
import { displayNode } from '@/shared/lib'

export function InheritedNotice({ node, declaredIn }: { node: string; declaredIn: string }) {
  const { pathname, search } = useLocation()
  const params = new URLSearchParams(search)
  params.set(searchParamKeys.node, declaredIn)
  return (
    <div role="note" aria-label="Inherited releases" className="flex flex-wrap items-center gap-x-2 gap-y-1 rounded-lg border border-border bg-muted/40 px-3 py-1.5 text-xs text-muted-foreground">
      <Info className="size-4 shrink-0" aria-hidden />
      <span>
        {displayNode(node)} plans against the releases of {displayNode(declaredIn)}: the list is changed there, items are still planned here.
      </span>
      <Link to={{ pathname, search: `?${params.toString()}` }} className="font-medium text-foreground underline underline-offset-2 hover:text-primary">
        Open {displayNode(declaredIn)}
      </Link>
    </div>
  )
}
