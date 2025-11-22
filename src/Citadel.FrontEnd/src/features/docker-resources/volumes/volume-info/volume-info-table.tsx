import { DockerVolumeResult } from '@/api/generated/api.types';
import { DataTable } from '@/components/ui/data-table';
import { byteTransform } from '@/lib/bytes.helper';
import { fromNow } from '@/lib/dayjs.helper';
import { ColumnDef } from '@tanstack/react-table';

const columns: ColumnDef<DockerVolumeResult>[] = [
  {
    accessorKey: 'driver',
    header: () => <span>Driver</span>,
    cell: ({ row }) => <span>{row.original.driver}</span>,
  },
  {
    accessorKey: 'scope',
    header: () => <span>Scope</span>,
    cell: ({ row }) => <span>{row.original.scope}</span>,
  },
  {
    accessorKey: 'created',
    header: () => <span>Created</span>,
    cell: ({ row }) => <span>{fromNow(new Date(row.original.createdAt).getTime())} </span>,
  },
  {
    accessorKey: 'size',
    header: () => <span>Size</span>,
    cell: ({ row }) => <span>{byteTransform(row.original.usageData?.size, 2)}</span>,
  },
];

export const VolumeInfoTable = ({ volume }: { volume: DockerVolumeResult | undefined }) => {
  if (!volume) return <></>;
  return (
    <div className="rounded-sm border p-1 shadow-xs">
      <DataTable columns={columns} data={volume ? [{ ...volume }] : []} isLoading={false} />
    </div>
  );
};
