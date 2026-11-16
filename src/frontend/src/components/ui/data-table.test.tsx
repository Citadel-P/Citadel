import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { StockFeatures, ColumnDef, Row } from '@tanstack/react-table';
import { DataTable } from './data-table';

type TestRow = {
  id: string;
  name: string;
  children?: TestRow[];
};

const columns: ColumnDef<StockFeatures, TestRow>[] = [
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

const NameCell = ({ row }: { row: Row<StockFeatures, TestRow> }) => (
  <div>
    {row.getCanExpand() && (
      <button type="button" onClick={row.getToggleExpandedHandler()}>
        Expand {row.original.name}
      </button>
    )}
    <span>{row.original.name}</span>
  </div>
);

it('shows initial loading without a false empty state and keeps rows visible during refresh', () => {
  const { rerender } = render(<DataTable columns={columns} data={[]} isLoading />);
  expect(screen.getByRole('status')).toHaveTextContent('Loading rows');
  expect(screen.getByRole('table')).toHaveAttribute('aria-busy', 'true');
  expect(screen.queryByText('No results.')).not.toBeInTheDocument();
  expect(screen.queryByRole('checkbox')).not.toBeInTheDocument();

  rerender(<DataTable columns={columns} data={[{ id: 'api', name: 'API' }]} isLoading />);
  expect(screen.getByText('API')).toBeVisible();
  expect(screen.queryByRole('status')).not.toBeInTheDocument();
  expect(screen.getByRole('table')).toHaveAttribute('aria-busy', 'false');

  rerender(<DataTable columns={columns} data={[]} isLoading={false} />);
  expect(screen.getByText('No results.')).toBeVisible();
  expect(screen.queryByRole('status')).not.toBeInTheDocument();
});

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
    expect(table.closest('[data-slot="table-container"]')).toHaveClass('rounded-lg', 'border', 'bg-card');

    screen.getAllByRole('columnheader').forEach((header) => {
      expect(header).toHaveClass('h-(--table-row-height)', 'text-xs', 'font-semibold');
    });

    screen.getAllByRole('cell').forEach((cell) => {
      expect(cell).toHaveClass('h-(--table-row-height)', 'text-sm');
    });

    expect(screen.getAllByRole('row')[1]).toHaveClass('even:bg-muted/15');
  });
});

describe('DataTable saved expansion', () => {
  beforeEach(() => localStorage.clear());

  const data = [{ id: 'parent', name: 'Stack', children: [{ id: 'child', name: 'Container' }] }];
  const view = (key = 'containers-expanded:one', rows = data) => (
    <DataTable
      columns={columns}
      data={rows}
      isLoading={false}
      getSubRows={(row) => row.children}
      expansionStorageKey={key}
    />
  );

  it('restores expansion after remounting without restoring selection, and remembers collapse', () => {
    const first = render(view());
    fireEvent.click(screen.getByRole('button', { name: 'Expand Stack' }));
    fireEvent.click(screen.getByRole('checkbox', { name: 'Select Container' }));
    first.unmount();

    const second = render(view());
    expect(screen.getByText('Container')).toBeVisible();
    expect(screen.getByRole('checkbox', { name: 'Select Container' })).not.toBeChecked();
    fireEvent.click(screen.getByRole('button', { name: 'Expand Stack' }));
    second.unmount();

    render(view());
    expect(screen.queryByText('Container')).not.toBeInTheDocument();
  });

  it('preserves expansion through filtered snapshots and keeps storage keys isolated', () => {
    const { rerender } = render(view());
    fireEvent.click(screen.getByRole('button', { name: 'Expand Stack' }));
    rerender(view('containers-expanded:one', []));
    rerender(view());
    expect(screen.getByText('Container')).toBeVisible();
    rerender(view('containers-expanded:two'));
    expect(screen.queryByText('Container')).not.toBeInTheDocument();
    rerender(view('alert-rules-expanded'));
    expect(screen.queryByText('Container')).not.toBeInTheDocument();
    rerender(view());
    expect(screen.getByText('Container')).toBeVisible();
  });

  it.each(['{invalid', 'null', '[]', '"expanded"', '{"parent":"true"}'])(
    'ignores malformed stored expansion: %s',
    (stored) => {
      localStorage.setItem('containers-expanded:one', stored);
      render(view());
      expect(screen.queryByText('Container')).not.toBeInTheDocument();
      fireEvent.click(screen.getByRole('button', { name: 'Expand Stack' }));
      expect(screen.getByText('Container')).toBeVisible();
    },
  );

  it('still expands when storage access is blocked', () => {
    const get = vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('Storage disabled');
    });
    const set = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('Storage disabled');
    });
    try {
      render(view());
      fireEvent.click(screen.getByRole('button', { name: 'Expand Stack' }));
      expect(screen.getByText('Container')).toBeVisible();
    } finally {
      get.mockRestore();
      set.mockRestore();
    }
  });
});
