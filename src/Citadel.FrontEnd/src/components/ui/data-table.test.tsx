import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { ColumnDef, Row } from '@tanstack/react-table';
import { DataTable } from './data-table';

type TestRow = {
  id: string;
  name: string;
  children?: TestRow[];
};

const columns: ColumnDef<TestRow>[] = [
  {
    id: 'select',
    cell: ({ row }) => (
      <input
        type="checkbox"
        aria-label={`Select ${row.original.name}`}
        checked={row.getIsSelected()}
        onChange={(event) => row.toggleSelected(event.target.checked)}
      />
    ),
  },
  {
    accessorKey: 'name',
    cell: ({ row }) => <NameCell row={row} />,
  },
];

const NameCell = ({ row }: { row: Row<TestRow> }) => (
  <div>
    {row.getCanExpand() && (
      <button type="button" onClick={row.getToggleExpandedHandler()}>
        Expand {row.original.name}
      </button>
    )}
    <span>{row.original.name}</span>
  </div>
);

describe('DataTable nested row selection', () => {
  it('reports an individually selected child row', async () => {
    const onSelectionChange = vi.fn();
    const child = { id: 'child', name: 'Container' };

    render(
      <DataTable
        columns={columns}
        data={[{ id: 'parent', name: 'Stack', children: [child] }]}
        isLoading={false}
        getSubRows={(row) => row.children}
        onSelectionChange={onSelectionChange}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Expand Stack' }));
    fireEvent.click(screen.getByRole('checkbox', { name: 'Select Container' }));

    await waitFor(() => expect(onSelectionChange).toHaveBeenLastCalledWith([child]));
  });

  it('can keep non-resource child rows out of resource selection', async () => {
    const onSelectionChange = vi.fn();
    const parent = { id: 'parent', name: 'Service', children: [{ id: 'child', name: 'Task' }] };

    render(
      <DataTable
        columns={columns}
        data={[parent]}
        isLoading={false}
        getSubRows={(row) => row.children}
        enableRowSelection={(row) => row.id === 'parent'}
        enableSubRowSelection={false}
        onSelectionChange={onSelectionChange}
      />,
    );

    fireEvent.click(screen.getByRole('checkbox', { name: 'Select Service' }));

    await waitFor(() => expect(onSelectionChange).toHaveBeenLastCalledWith([parent]));
  });
});

describe('DataTable styling', () => {
  it('uses one typography and spacing contract for every column', () => {
    render(<DataTable columns={columns} data={[{ id: 'stack', name: 'Stack' }]} isLoading={false} />);

    const table = screen.getByRole('table');
    expect(table.closest('[data-slot="table-container"]')).toHaveClass('rounded-md', 'border', 'p-1');

    screen.getAllByRole('columnheader').forEach((header) => {
      expect(header).toHaveClass('h-11', 'text-xs', 'font-semibold');
    });

    screen.getAllByRole('cell').forEach((cell) => {
      expect(cell).toHaveClass('h-12', 'text-sm');
    });

    expect(screen.getAllByRole('row')[1]).toHaveClass('even:bg-muted/15');
  });
});
