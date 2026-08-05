import { SwarmConfigView } from '@/api/generated/api.types';
import { TimestampCell } from '@/components/custom/timestamp-cell';
import { DataTable } from '@/components/ui/data-table';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { ColumnDef } from '@tanstack/react-table';
import { useMemo } from 'react';

export const ConfigInfoTable = ({ config }: { config: SwarmConfigView }) => {
  const formatDateTime = useProfileDateTimeFormatter();
  const columns = useMemo<ColumnDef<SwarmConfigView>[]>(
    () => [
      {
        accessorKey: 'templatingDriver',
        header: 'Templating driver',
        cell: ({ row }) => row.original.templatingDriver ?? '-',
      },
      { accessorKey: 'versionIndex', header: 'Version' },
      {
        accessorKey: 'createdAt',
        header: 'Created',
        cell: ({ row }) => <TimestampCell value={row.original.createdAt} formatDateTime={formatDateTime} />,
      },
      {
        accessorKey: 'updatedAt',
        header: 'Updated',
        cell: ({ row }) => <TimestampCell value={row.original.updatedAt} formatDateTime={formatDateTime} />,
      },
    ],
    [formatDateTime],
  );

  return (
    <div className="rounded-sm border p-1 shadow-xs">
      <DataTable columns={columns} data={[config]} isLoading={false} />
    </div>
  );
};
