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
});
