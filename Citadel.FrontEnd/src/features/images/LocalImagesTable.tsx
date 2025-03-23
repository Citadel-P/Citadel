import { DataTable } from '@/components/ui/data-table';
import { ImageView } from '@/api/_generated';
import SortableCell from '@/components/ui/SortableCell';
import { ColumnDef } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { Link } from 'react-router';
import { useGETInternalImages } from './hooks/useGETAllLocalImages';
import DropdownTableMenu from './DropdownTableMenu';
import { useContextSelector } from 'use-context-selector';
import { ImagesContext } from './ImagesProvider';
import { truncate } from '@/lib/truncate';
import { useCopyToClipboard } from '@/hooks/useCopyToClipboard';
import { CheckCheck, Clipboard } from 'lucide-react';
import { useEffect } from 'react';
import { AppContext } from '@/AppProvider';
import { fromNow } from '@/lib/dayjs.helper';
import { byteTransform } from '@/lib/bytes.helper';

const columns: ColumnDef<ImageView>[] = [
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
        aria-label="Select registry"
      />
    ),
    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: 'name',
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => (
      <Link to={`../registries/edit/${row.original.id}`} className="hover:underline">
        {truncate(row.original.name ?? '', 35, 'right')}
      </Link>
    ),
    sortingFn: (rowA: any, rowB: any, _columnId: any): number => {
      return rowA.original.name < rowB.original.name ? 1 : -1;
    },
  },
  {
    accessorKey: 'tag',
    header: ({ column }) => <SortableCell cellName="Tag" column={column} />,
    cell: ({ row }) => <div>{row.original.tag}</div>,
    sortingFn: (rowA: any, rowB: any, _columnId: any): number => {
      return rowA.original.discriminator < rowB.original.discriminator ? 1 : -1;
    },
  },
  {
    accessorKey: 'id',
    header: ({ column }) => <SortableCell cellName="Image Id" column={column} />,
    cell: ({ row }) => <ImageIdRow image={row.original} />,
    sortingFn: (rowA: any, rowB: any, _columnId: any): number => {
      return rowA.original.url < rowB.original.url ? 1 : -1;
    },
  },
  {
    accessorKey: 'created',
    header: ({ column }) => <SortableCell cellName="Created" column={column} />,
    cell: ({ row }) => <span className="text-[13px]">{fromNow(new Date(row.original.created * 1000).getTime())}</span>,
    sortingFn: (rowA: any, rowB: any, _columnId: any): number => {
      return rowA.original.url < rowB.original.url ? 1 : -1;
    },
  },
  {
    accessorKey: 'size',
    header: ({ column }) => <SortableCell cellName="Size" column={column} />,
    cell: ({ row }) => <span className="text-[13px]">{byteTransform(row.original.size, 2)}</span>,
    sortingFn: (rowA: any, rowB: any, _columnId: any): number => {
      return rowA.original.url < rowB.original.url ? 1 : -1;
    },
  },
  {
    id: 'actions',
    cell: ({ row }) => {
      return (
        <div className="text-center">
          <DropdownTableMenu image={row.original} />
        </div>
      );
    },
  },
];

const ImageIdRow = ({ image }: { image: ImageView }) => {
  const [copiedWinCmd, copyWinCmdToClipboard] = useCopyToClipboard(5000);

  return (
    <div className="flex gap-0.5 items-center">
      <div>{truncate(image.id?.split(':').at(1) ?? '', 12, 'right', true)}</div>
      <button
        className="rounded-full invisible group-hover/trow:visible ml-1 px-1.5 py-1.5 bg-foreground/5 hover:bg-foreground/10 text-sm font-semibold"
        onClick={() => copyWinCmdToClipboard(image.id ?? '')}>
        {copiedWinCmd ? <CheckCheck className="w-3 h-3 text-green-500" /> : <Clipboard className="w-3 h-3 " />}
      </button>
    </div>
  );
};

export default function LocalImagesTable() {
  const currentPlatform = useContextSelector(AppContext, (v) => v?.currentPlatform);
  const { data, isLoading, isSuccess, error } = useGETInternalImages(currentPlatform?.id);
  const setSelectedRowIds = useContextSelector(ImagesContext, (v) => v?.setSelectedRowIds)!;
  const setLocalImages = useContextSelector(ImagesContext, (v) => v?.setLocalImages)!;

  useEffect(() => {
    if (isSuccess && data?.data.images) {
      setLocalImages(data.data.images);
    }
    return () => {
      setLocalImages([]);
      setSelectedRowIds([]);
    };
  }, [data, isSuccess, setLocalImages, setSelectedRowIds]);

  return (
    <div className="flex flex-col gap-3">
      <DataTable
        columns={columns}
        data={data?.data.images ?? []}
        isLoading={isLoading}
        onSelectionChange={setSelectedRowIds}
      />
      <div className="text-muted-foreground text-xs font-normal ">
        {data?.data.images?.length && (
          <span>
            Showing {data?.data.images?.length} of {data?.data.images?.length} image(s)
          </span>
        )}
      </div>
    </div>
  );
}
