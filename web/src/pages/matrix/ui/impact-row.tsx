import { ItemCard } from '@/entities/item'
import { cn } from '@/shared/lib'
import { Draggable, DropZone } from '@/shared/ui'
import { cellId, isQuickWin, type Matrix } from '../model/placement'

export function ImpactRow({ impact, matrix }: { impact: string; matrix: Matrix }) {
  return (
    <>
      <div className="self-center text-right text-xs font-semibold tracking-wide text-muted-foreground uppercase">
        {impact}
      </div>
      {matrix.sizes.map((size) => (
        <DropZone
          key={size}
          id={cellId(impact, size)}
          label={`impact ${impact}, size ${size}`}
          className={cn(
            'flex min-h-28 flex-col gap-2 rounded-lg border border-border bg-muted/30 p-2',
            isQuickWin(matrix, impact, size) && 'border-green-500/50 bg-green-500/10',
          )}
        >
          {(matrix.cells.get(cellId(impact, size)) ?? []).map((item) => (
            <Draggable key={item.path} id={item.path}>
              <ItemCard item={item} showNode />
            </Draggable>
          ))}
        </DropZone>
      ))}
    </>
  )
}
