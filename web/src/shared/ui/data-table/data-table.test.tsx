import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { DataTable, type Column } from './data-table'

interface Fruit {
  name: string
  weight: number | null
}

const fruits: Fruit[] = [
  { name: 'Pear', weight: 180 },
  { name: 'Apple', weight: null },
  { name: 'Fig', weight: 50 },
]

const columns: Column<Fruit>[] = [
  { id: 'name', header: 'Name', cell: (fruit) => fruit.name, sortValue: (fruit) => fruit.name },
  { id: 'weight', header: 'Weight', cell: (fruit) => fruit.weight ?? '—', sortValue: (fruit) => fruit.weight },
  { id: 'actions', header: 'Actions', cell: () => null },
]

function renderTable(table = 'fruits') {
  return render(<DataTable rows={fruits} columns={columns} rowKey={(fruit) => fruit.name} storageKey={table} label="Fruits" />)
}

const names = () =>
  screen
    .getAllByRole('row')
    .slice(1)
    .map((row) => within(row).getAllByRole('cell')[0]?.textContent)

describe('DataTable', () => {
  it('keeps the given order until a header is clicked', () => {
    renderTable()
    expect(names()).toEqual(['Pear', 'Apple', 'Fig'])
  })

  it('cycles a column through ascending, descending and the given order', async () => {
    renderTable()
    const weight = screen.getByRole('button', { name: 'Weight' })
    await userEvent.click(weight)
    expect(names()).toEqual(['Fig', 'Pear', 'Apple'])
    expect(screen.getByRole('columnheader', { name: 'Weight' })).toHaveAttribute('aria-sort', 'ascending')
    await userEvent.click(weight)
    expect(names()).toEqual(['Pear', 'Fig', 'Apple'])
    expect(screen.getByRole('columnheader', { name: 'Weight' })).toHaveAttribute('aria-sort', 'descending')
    await userEvent.click(weight)
    expect(names()).toEqual(['Pear', 'Apple', 'Fig'])
  })

  it('offers no sort on a column without a sort value', () => {
    renderTable()
    expect(screen.queryByRole('button', { name: 'Actions' })).not.toBeInTheDocument()
    expect(screen.getByRole('columnheader', { name: 'Actions' })).not.toHaveAttribute('aria-sort')
  })

  it('remembers the sort of a table across visits', async () => {
    const first = renderTable()
    await userEvent.click(screen.getByRole('button', { name: 'Name' }))
    first.unmount()
    renderTable()
    expect(names()).toEqual(['Apple', 'Fig', 'Pear'])
    expect(localStorage.getItem('ariane.sort.fruits')).toBe('{"column":"name","direction":"asc"}')
  })

  it('sorts every table of one key together, and no other', async () => {
    render(
      <>
        <DataTable rows={fruits} columns={columns} rowKey={(fruit) => fruit.name} storageKey="fruits" label="First" />
        <DataTable rows={fruits} columns={columns} rowKey={(fruit) => fruit.name} storageKey="fruits" label="Second" />
        <DataTable rows={fruits} columns={columns} rowKey={(fruit) => fruit.name} storageKey="baskets" label="Third" />
      </>,
    )
    await userEvent.click(within(screen.getByRole('table', { name: 'First' })).getByRole('button', { name: 'Name' }))
    const firstNames = (label: string) =>
      within(screen.getByRole('table', { name: label }))
        .getAllByRole('row')
        .slice(1)
        .map((row) => within(row).getAllByRole('cell')[0]?.textContent)
    expect(firstNames('Second')).toEqual(['Apple', 'Fig', 'Pear'])
    expect(firstNames('Third')).toEqual(['Pear', 'Apple', 'Fig'])
  })
})
