import { ItemCard } from '@/entities/item'
import { cn } from '@/shared/lib'
import { Draggable, DropZone } from '@/shared/ui'
import { cellId, isQuickWin, type Matrix } from '../model/placement'
import './matrix.css'

export function ImpactRow({ impact, matrix }: { impact: string; matrix: Matrix }) {
  return (
    <>
      <div className="impact-row__label">
        {impact}
      </div>
      {matrix.sizes.map((size) => (
        <DropZone
          key={size}
          id={cellId(impact, size)}
          label={`impact ${impact}, size ${size}`}
          className={cn(
            'impact-row__cell',
            isQuickWin(matrix, impact, size) && 'impact-row__cell--quick-win',
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
