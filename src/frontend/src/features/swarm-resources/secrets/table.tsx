import { SwarmSecretView } from '@/api/generated/api.types';
import SortableCell from '@/components/custom/sortable-cell';
import { TimestampCell } from '@/components/custom/timestamp-cell';
import { Badge } from '@/components/ui/badge';
import { DataTable } from '@/components/ui/data-table';
import { Checkbox } from '@/components/ui/checkbox';
import { StateIndicator } from '@/components/custom/state-indicator';
import { RowActionMenu } from '@/components/custom/dropdown-with-dialog';
import { useSelectedResources } from '@/lib/atoms';
import { DropdownActionComponent } from '@/pages/types';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { ColumnDef } from '@tanstack/react-table';
import { useMemo } from 'react';
import { Link, useParams } from 'react-router';
import { displayList } from '../shared';
import { SwarmResourceEditDialog } from '../resource-edit-dialog';
import { useState } from 'react';

export const SecretsTable = ({
  items,
  actions,
  isLoading,
}: {
  items: SwarmSecretView[];
  actions: Record<string, DropdownActionComponent<SwarmSecretView>>;
  isLoading: boolean;
}) => {
  const { platformId = '' } = useParams<{ platformId: string }>();
  const [, setSelectedResources] = useSelectedResources<SwarmSecretView>('Secret');
  const [editing, setEditing] = useState<SwarmSecretView | null>(null);
  const formatDateTime = useProfileDateTimeFormatter();
  const columns = useMemo<ColumnDef<SwarmSecretView>[]>(
    () => [
      {
        id: 'select',
        header: ({ table }) => (
          <Checkbox
            checked={table.getIsAllPageRowsSelected() || (table.getIsSomePageRowsSelected() && 'indeterminate')}
            onCheckedChange={(value) => table.toggleAllPageRowsSelected(!!value)}
            aria-label="Select all secrets"
          />
        ),
        cell: ({ row }) => (
          <Checkbox
            checked={row.getIsSelected()}
            onCheckedChange={(value) => row.toggleSelected(!!value)}
            aria-label={`Select ${row.original.name}`}
          />
        ),
        enableSorting: false,
        enableHiding: false,
      },
      {
        accessorKey: 'name',
        header: ({ column }) => <SortableCell cellName="Name" column={column} />,
        cell: ({ row }) => (
          <div className="flex min-w-0 items-center gap-2">
            <StateIndicator value={row.original.inUse} />
            <Link className="table-link truncate" to={`/platforms/${platformId}/secrets/${row.original.id}`}>
              {row.original.name || row.original.id.slice(0, 12)}
            </Link>
            {row.original.isStale && <Badge variant="secondary">Stale</Badge>}
          </div>
        ),
      },
      {
        accessorKey: 'driver',
        header: ({ column }) => <SortableCell cellName="Driver" column={column} />,
        cell: ({ row }) => row.original.driver ?? 'Swarm',
      },
      { id: 'services', header: 'Used by', cell: ({ row }) => displayList(row.original.serviceNames) },
      {
        accessorKey: 'createdAt',
        header: ({ column }) => <SortableCell cellName="Created" column={column} />,
        cell: ({ row }) => <TimestampCell value={row.original.createdAt} formatDateTime={formatDateTime} />,
      },
      {
        id: 'actions',
        cell: ({ row }) => (
          <RowActionMenu
            resource={row.original}
            actions={actions}
            onAction={({ key }) => key === 'edit' && setEditing(row.original)}
          />
        ),
        enableSorting: false,
        enableHiding: false,
      },
    ],
    [actions, formatDateTime, platformId],
  );
  return (
    <>
      <DataTable
        columns={columns}
        data={items}
        isLoading={isLoading}
        onSelectionChange={setSelectedResources}
        emptyState={{ title: 'No secrets found.', description: 'No secrets were returned by the Swarm manager.' }}
      />
      {editing && (
        <SwarmResourceEditDialog
          resource={editing}
          kind="secret"
          open
          onOpenChange={(open) => !open && setEditing(null)}
        />
      )}
    </>
  );
};
