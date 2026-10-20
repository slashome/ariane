import { enumValues } from '@/entities/schema'
import { isClosed, isWorkItem } from '@/entities/item'
import type { Item, Schema } from '@/shared/api'

export interface Matrix {
  sizes: string[]
  impacts: string[]
  cells: Map<string, Item[]>
  unestimated: Item[]
}

export function cellId(impact: string, size: string): string {
  return `cell:${impact}:${size}`
}

export function parseCellId(id: string): { impact: string; size: string } | null {
  const [prefix, impact, size] = id.split(':')
  return prefix === 'cell' && impact && size ? { impact, size } : null
}

export function isQuickWin(matrix: Pick<Matrix, 'sizes' | 'impacts'>, impact: string, size: string): boolean {
  return matrix.sizes.slice(0, 2).includes(size) && matrix.impacts[0] === impact
}

export function placeItems(items: Item[], schema: Schema | undefined, includeClosed: boolean): Matrix {
  const sizes = enumValues(schema, 'size')
  const impacts = [...enumValues(schema, 'impact')].reverse()
  const cells = new Map<string, Item[]>()
  const unestimated: Item[] = []
  for (const item of items) {
    if (item.archived || !isWorkItem(item) || (!includeClosed && isClosed(item.category))) {
      continue
    }
    if (item.size !== null && item.impact !== null && sizes.includes(item.size) && impacts.includes(item.impact)) {
      const id = cellId(item.impact, item.size)
      cells.set(id, [...(cells.get(id) ?? []), item])
    } else {
      unestimated.push(item)
    }
  }
  return { sizes, impacts, cells, unestimated }
}
