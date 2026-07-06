import { OidcProviderView } from '@/api/generated/api.types';
import { ContentCard } from '@/components/custom/content-card';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import SortableCell from '@/components/custom/sortable-cell';
import { StateIndicator } from '@/components/custom/state-indicator';
import { Checkbox } from '@/components/ui/checkbox';
import { DataTable } from '@/components/ui/data-table';
import { useSelectedResources } from '@/lib/atoms';
import { ActionData } from '@/pages/types';
import { ColumnDef } from '@tanstack/react-table';
import { useMemo } from 'react';
import { Link } from 'react-router';

export function OidcProvidersTable({
  items,
  actions,
  isLoading,
}: {
  items: OidcProviderView[];
  isLoading: boolean;
  actions: Record<
    string,
    React.FC<{ resource: OidcProviderView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >;
}) {
  const [, setSelectedResources] = useSelectedResources<OidcProviderView>('OidcProvider');
  const cols = useMemo(() => columns(actions ?? {}), [actions]);

  return (
    <ContentCard>
      <DataTable columns={cols} data={items} isLoading={isLoading} onSelectionChange={setSelectedResources} />
    </ContentCard>
  );
}

const columns = (
  actions: Record<
    string,
    React.FC<{ resource: OidcProviderView; onAction?: (actionKey: string, actionData?: ActionData) => void }>
  >,
): ColumnDef<OidcProviderView>[] => [
  {
    id: 'select',
    header: ({ table }) => (
      <Checkbox
        checked={table.getIsAllPageRowsSelected() || (table.getIsSomePageRowsSelected() && 'indeterminate')}
        onCheckedChange={(value) => table.toggleAllPageRowsSelected(!!value)}
        aria-label="Select all"
      />
    ),
    cell: ({ row }) => (
      <Checkbox
        checked={row.getIsSelected()}
        onCheckedChange={(value) => row.toggleSelected(!!value)}
        aria-label="Select OIDC provider"
      />
    ),
    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: 'displayName',
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => <ProviderNameRow provider={row.original} />,
    sortingFn: (rowA, rowB) => rowA.original.displayName.localeCompare(rowB.original.displayName),
  },
  {
    accessorKey: 'issuer',
    header: ({ column }) => <SortableCell cellName="Issuer" column={column} />,
    cell: ({ row }) => <span className="block max-w-85 truncate text-[13px]">{row.original.issuer}</span>,
    sortingFn: (rowA, rowB) => rowA.original.issuer.localeCompare(rowB.original.issuer),
  },
  {
    accessorKey: 'clientId',
    header: ({ column }) => <SortableCell cellName="Client ID" column={column} />,
    cell: ({ row }) => <span className="block max-w-60 truncate text-[13px]">{row.original.clientId}</span>,
    sortingFn: (rowA, rowB) => rowA.original.clientId.localeCompare(rowB.original.clientId),
  },
  {
    accessorKey: 'scopes',
    header: ({ column }) => <SortableCell cellName="Scopes" column={column} />,
    cell: ({ row }) => <span className="block max-w-64 truncate text-[13px]">{row.original.scopes}</span>,
    sortingFn: (rowA, rowB) => rowA.original.scopes.localeCompare(rowB.original.scopes),
  },
  {
    id: 'actions',
    cell: ({ row }) => <RowActionMenu resource={row.original} actions={actions} />,
  },
];

const ProviderNameRow = ({ provider }: { provider: OidcProviderView }) => (
  <div className="flex min-w-0 items-center gap-1">
    <div className="mt-1.5">
      <StateIndicator value={provider.enabled} enableLabel={true} />
    </div>
    <div className="min-w-0">
      <Link to={`../oidc-providers/edit/${provider.id}`} className="truncate text-sm font-medium hover:underline">
        {provider.displayName}
      </Link>
      <div className="truncate text-xs text-muted-foreground">{provider.name}</div>
    </div>
  </div>
);
