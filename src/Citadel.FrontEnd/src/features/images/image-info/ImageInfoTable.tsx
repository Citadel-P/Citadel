import { InspectImageView } from '@/api/_generated';
import { DataTable } from '@/components/ui/data-table';
import { RegistryDisplay } from '@/components/ui/RegistryDisplay';
import { byteTransform } from '@/lib/bytes.helper';
import { fromNow } from '@/lib/dayjs.helper';
import { ColumnDef } from '@tanstack/react-table';

const columns: ColumnDef<InspectImageView>[] = [
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
    cell: ({ row }) => <span>{fromNow(new Date(row.original.created).getTime())} </span>,
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
  if (!image) return <></>;
  return (
    <div className="flex flex-col gap-3">
      <DataTable columns={columns} data={image ? [{ ...image }] : []} isLoading={false} />
    </div>
  );
};
