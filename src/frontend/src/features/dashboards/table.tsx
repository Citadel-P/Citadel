import type { ColumnDef, StockFeatures } from '@tanstack/react-table';
import { House } from 'lucide-react';
import { createContext, useContext } from 'react';
import { Link } from 'react-router';
import type { DashboardSummaryView } from '@/api/generated/api.types';
import { ActionBar } from '@/components/custom/action-bar';
import SortableCell from '@/components/custom/sortable-cell';
import { StateBadge } from '@/components/custom/state-badge';
import { Checkbox } from '@/components/ui/checkbox';
import { DataTable } from '@/components/ui/data-table';
import { useSelectedResources } from '@/lib/atoms';
import { useRead } from '@/lib/hooks';
import { DashboardRowActions, dashboardGroupActions } from './actions';
import { audiences } from './visibility';

const HomeDashboardId = createContext<string | null | undefined>(undefined);

function HomeCell({ id }: { id: string }) {
  const homeId = useContext(HomeDashboardId);
  return id === homeId ? (
    <StateBadge value={true} label="Current home" indicator={<House aria-hidden="true" />} />
  ) : null;
}

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
    cell: ({ row }) => (
      <StateBadge kind="visibility" value={row.original.visibility} label={audiences[row.original.visibility]} />
    ),
  },
  {
    id: 'home',
    header: 'Home',
    cell: ({ row }) => <HomeCell id={row.id} />,
  },
  { id: 'actions', cell: ({ row }) => <DashboardRowActions board={row.original} /> },
];

export function DashboardsTable({ items, isLoading }: { items: DashboardSummaryView[]; isLoading: boolean }) {
  const [, setSelected] = useSelectedResources<DashboardSummaryView>('Dashboard');
  const home = useRead('getDashboardDefault', undefined, { enabled: items.length > 0, staleTime: 0 });
  const homeId = home.isError ? undefined : home.data?.data.dashboardId;
  return (
    <>
      <HomeDashboardId.Provider value={homeId}>
        <DataTable
          columns={columns}
          data={items}
          isLoading={isLoading}
          emptyState={{ title: 'No dashboards found.' }}
          onSelectionChange={setSelected}
        />
      </HomeDashboardId.Provider>
      <ActionBar type="Dashboard" items={items} actions={dashboardGroupActions} />
    </>
  );
}
