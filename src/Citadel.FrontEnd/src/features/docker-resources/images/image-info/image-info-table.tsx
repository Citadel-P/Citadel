import { InspectImageView } from '@/api/generated/api.types';
import { DataTable } from '@/components/ui/data-table';
import { RegistryDisplay } from '@/components/custom/registry-display';
import { byteTransform } from '@/lib/bytes.helper';
import { ColumnDef } from '@tanstack/react-table';
import { TimestampCell } from '@/components/custom/timestamp-cell';
import type { DateTimeFormatter } from '@/lib/date-time';
import { useProfileDateTimeFormatter } from '@/lib/use-profile-date-time';
import { useMemo } from 'react';

const columns = (formatDateTime: DateTimeFormatter): ColumnDef<InspectImageView>[] => [
  {
    accessorKey: 'os',
    header: () => <span>Os</span>,
    cell: ({ row }) => <span>{row.original.os}</span>,
  },
  {
    accessorKey: 'architecture',
    header: () => <span>Architecture</span>,
    cell: ({ row }) => <span>{row.original.architecture}</span>,
  },
  {
    accessorKey: 'registry',
    header: () => <span>Registry</span>,
    cell: ({ row }) => <RegistryDisplay registry={row.original.registry ?? undefined} />,
  },
  {
    accessorKey: 'created',
    header: () => <span>Created</span>,
    cell: ({ row }) => <TimestampCell value={row.original.created} formatDateTime={formatDateTime} />,
  },
  {
    accessorKey: 'size',
    header: () => <span>Size</span>,
    cell: ({ row }) => <span>{byteTransform(row.original.size, 2)}</span>,
  },
  {
    accessorKey: 'expose',
    header: () => <span>Expose</span>,
    cell: ({ row }) => (
      <div className="text-foreground gap-2 flex flex-wrap items-center">
        {row.original.exposedPorts.map((c) => (
          <span key={c}>{c}</span>
        ))}
      </div>
    ),
  },
];

export const ImageInfoTable = ({ image }: { image: InspectImageView | undefined }) => {
  const formatDateTime = useProfileDateTimeFormatter();
  const cols = useMemo(() => columns(formatDateTime), [formatDateTime]);

  if (!image) return <></>;
  return (
    <div className="rounded-sm border p-1 shadow-xs">
      <DataTable columns={cols} data={[{ ...image }]} isLoading={false} />
    </div>
  );
};
