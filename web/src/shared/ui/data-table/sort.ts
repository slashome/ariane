export type SortValue = string | number | boolean | null | undefined

export type SortDirection = 'asc' | 'desc'

export interface TableSort {
  column: string
  direction: SortDirection
}

type Filled = string | number | boolean

const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' })

function isFilled(value: SortValue): value is Filled {
  return value !== null && value !== undefined && value !== ''
}

function compareFilled(left: Filled, right: Filled): number {
  if (typeof left === 'string' || typeof right === 'string') {
    return collator.compare(String(left), String(right))
  }
  return Number(left) - Number(right)
}

export function sortRows<Row>(rows: Row[], valueOf: (row: Row) => SortValue, direction: SortDirection): Row[] {
  const sign = direction === 'asc' ? 1 : -1
  return rows
    .map((row, index) => ({ row, index, value: valueOf(row) }))
    .sort((left, right) => {
      const order =
        isFilled(left.value) && isFilled(right.value)
          ? sign * compareFilled(left.value, right.value)
          : Number(!isFilled(left.value)) - Number(!isFilled(right.value))
      return order || left.index - right.index
    })
    .map(({ row }) => row)
}

export function nextSort(current: TableSort | null, column: string): TableSort | null {
  if (current?.column !== column) {
    return { column, direction: 'asc' }
  }
  return current.direction === 'asc' ? { column, direction: 'desc' } : null
}

export function parseSort(raw: string | null): TableSort | null {
  if (raw === null) {
    return null
  }
  try {
    const parsed: unknown = JSON.parse(raw)
    if (
      typeof parsed === 'object' &&
      parsed !== null &&
      'column' in parsed &&
      'direction' in parsed &&
      typeof parsed.column === 'string' &&
      (parsed.direction === 'asc' || parsed.direction === 'desc')
    ) {
      return { column: parsed.column, direction: parsed.direction }
    }
    return null
  } catch {
    return null
  }
}
