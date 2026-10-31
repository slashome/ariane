import { DataTable as LumenDataTable } from '@slashome/lumen/react'
import type { ComponentProps } from 'react'

export function DataTable<Row>({ storageKey, ...props }: ComponentProps<typeof LumenDataTable<Row>>) {
  return <LumenDataTable<Row> storageKey={`ariane.sort.${storageKey}`} {...props} />
}
