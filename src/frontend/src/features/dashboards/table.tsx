import type { ColumnDef, StockFeatures } from '@tanstack/react-table';
import { Link } from 'react-router';
import type { DashboardSummaryView } from '@/api/generated/api.types';
import { ActionBar } from '@/components/custom/action-bar';
import SortableCell from '@/components/custom/sortable-cell';
import { Badge } from '@/components/ui/badge';
import { Checkbox } from '@/components/ui/checkbox';
import { DataTable } from '@/components/ui/data-table';
import { useSelectedResources } from '@/lib/atoms';
import { DashboardRowActions, dashboardGroupActions } from './actions';
import { audiences } from './visibility';

const columns: ColumnDef<StockFeatures, DashboardSummaryView>[] = [
  {
    id: 'select',
    header: ({ table }) => (
      <Checkbox
        aria-label="Select all dashboards on this page"
        checked={table.getIsAllPageRowsSelected() || (table.getIsSomePageRowsSelected() && 'indeterminate')}
        onCheckedChange={(v) => table.toggleAllPageRowsSelected(!!v)}
      />
    ),
    cell: ({ row }) => (
      <Checkbox
        aria-label={`Select ${row.original.name}`}
        checked={row.getIsSelected()}
        onCheckedChange={(v) => row.toggleSelected(!!v)}
      />
    ),
    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: 'name',
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => (
      <Link className="table-link" to={'/dashboards/' + row.id}>
        {row.original.name}
      </Link>
    ),
  },
  {
    accessorKey: 'visibility',
    header: 'Who can read',
    cell: ({ row }) => <Badge variant="outline">{audiences[row.original.visibility]}</Badge>,
  },
  { id: 'actions', cell: ({ row }) => <DashboardRowActions board={row.original} /> },
];

export function DashboardsTable({ items, isLoading }: { items: DashboardSummaryView[]; isLoading: boolean }) {
  const [, setSelected] = useSelectedResources<DashboardSummaryView>('Dashboard');
  return (
    <>
      <DataTable
        columns={columns}
        data={items}
        isLoading={isLoading}
        emptyState={{ title: 'No dashboards found.' }}
        onSelectionChange={setSelected}
      />
      <ActionBar type="Dashboard" items={items} actions={dashboardGroupActions} />
    </>
  );
}
