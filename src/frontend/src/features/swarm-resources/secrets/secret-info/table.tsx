import { SwarmSecretView } from '@/api/generated/api.types';
import { Timestamp } from '@/components/custom/timestamp';
import { DataTable } from '@/components/ui/data-table';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { StockFeatures, ColumnDef } from '@tanstack/react-table';
import { useMemo } from 'react';

export const SecretInfoTable = ({ secret }: { secret: SwarmSecretView }) => {
  const formatDateTime = useProfileDateTimeFormatter();
  const columns = useMemo<ColumnDef<StockFeatures, SwarmSecretView>[]>(
    () => [
      { accessorKey: 'driver', header: 'Driver', cell: ({ row }) => row.original.driver ?? 'Swarm' },
      { accessorKey: 'versionIndex', header: 'Version' },
      {
        accessorKey: 'createdAt',
        header: 'Created',
        cell: ({ row }) => <Timestamp value={row.original.createdAt} formatDateTime={formatDateTime} />,
      },
      {
        accessorKey: 'updatedAt',
        header: 'Updated',
        cell: ({ row }) => <Timestamp value={row.original.updatedAt} formatDateTime={formatDateTime} />,
      },
    ],
    [formatDateTime],
  );

  return (
    <div className="rounded-sm border p-1 shadow-xs">
      <DataTable columns={columns} data={[secret]} isLoading={false} />
    </div>
  );
};
