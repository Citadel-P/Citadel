import { SwarmSecretView } from '@/api/generated/api.types';
import SortableCell from '@/components/custom/sortable-cell';
import { TimestampCell } from '@/components/custom/timestamp-cell';
import { Badge } from '@/components/ui/badge';
import { DataTable } from '@/components/ui/data-table';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { ColumnDef } from '@tanstack/react-table';
import { useMemo } from 'react';
import { Link, useParams } from 'react-router';
import { displayList } from '../shared';

export const SecretsTable = ({ items, isLoading }: { items: SwarmSecretView[]; isLoading: boolean }) => {
  const { platformId = '' } = useParams<{ platformId: string }>();
  const formatDateTime = useProfileDateTimeFormatter();
  const columns = useMemo<ColumnDef<SwarmSecretView>[]>(
    () => [
      {
        accessorKey: 'name',
        header: ({ column }) => <SortableCell cellName="Name" column={column} />,
        cell: ({ row }) => (
          <div className="flex min-w-0 items-center gap-2">
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
    ],
    [formatDateTime, platformId],
  );
  return (
    <DataTable
      columns={columns}
      data={items}
      isLoading={isLoading}
      emptyState={{ title: 'No secrets found.', description: 'No secrets were returned by the Swarm manager.' }}
    />
  );
};
