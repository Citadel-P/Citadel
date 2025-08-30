import { HistoryImageResult, InspectImageResult } from '@/api/_generated';
import { DataTable } from '@/components/ui/data-table';
import { byteTransform } from '@/lib/bytes.helper';
import { truncate } from '@/lib/truncate';
import { ColumnDef } from '@tanstack/react-table';

const columns: ColumnDef<HistoryImageResult & { rowId: string }>[] = [
  {
    accessorKey: 'stage',
    header: () => <span>Stage</span>,
    cell: ({ row }) => <span>{truncate(row.original.createdBy, 120)} </span>,
  },
  {
    accessorKey: 'size',
    header: () => <span>Size</span>,
    cell: ({ row }) => <span>{byteTransform(row.original.size, 2)}</span>,
  },
];

export const ImageLayerTable = ({ image }: { image: InspectImageResult | undefined }) => {
  if (!image) return <></>;
  const layersWithId = (image.layers ?? []).map((layer, index) => ({
    ...layer,
    rowId: index.toString(),
  }));
  return (
    <div className="flex flex-col gap-3">
      <DataTable columns={columns} data={layersWithId} isLoading={false} getRowId={(row) => row.rowId}  />
    </div>
  );
};
