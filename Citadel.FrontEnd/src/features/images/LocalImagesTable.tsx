import { DataTable } from '@/components/ui/data-table';
import { ImageView } from '@/api/_generated';
import SortableCell from '@/components/ui/SortableCell';
import { ColumnDef } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { useGETAllLocalImages } from './hooks/useGETAllLocalImages';
import DropdownTableMenu from './DropdownTableMenu';
import { useContextSelector } from 'use-context-selector';
import { ImagesContext } from './ImagesProvider';
import { truncate } from '@/lib/truncate';
import { useCopyToClipboard } from '@/hooks/useCopyToClipboard';
import { CheckCheck, Clipboard } from 'lucide-react';
import { useEffect, useCallback, useMemo, memo } from 'react';
import { fromNow } from '@/lib/dayjs.helper';
import { byteTransform } from '@/lib/bytes.helper';
import { DeleteLocalImageDialog } from './dialogs/DeleteLocalImageDialog';
import { AppContext } from '@/AppProvider';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';

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
        aria-label="Select image"
      />
    ),
    enableSorting: false,
    enableHiding: false,
  },
  {
    accessorKey: 'name',
    header: ({ column }) => <SortableCell cellName="Name" column={column} />,
    cell: ({ row }) => (
      <div className="flex items-center whitespace-nowrap">
        <div className="flex items-center">
          <ImageStatusTooltip inUse={row.original.isInUse} />
        </div>
        <span>{truncate(row.original.name ?? '', 35, 'right')}</span>
      </div>
    ),
    sortingFn: (rowA: any, rowB: any): number => rowA.original?.name?.localeCompare(rowB.original?.name),
  },
  {
    accessorKey: 'tag',
    header: ({ column }) => <SortableCell cellName="Tag" column={column} />,
    cell: ({ row }) => <div>{row.original.tag}</div>,
    sortingFn: (rowA: any, rowB: any): number => rowA.original?.tag?.localeCompare(rowB.original?.tag),
  },
  {
    accessorKey: 'id',
    header: ({ column }) => <SortableCell cellName="Image Id" column={column} />,
    cell: ({ row }) => <ImageIdRow image={row.original} />,
    sortingFn: (rowA: any, rowB: any): number => rowA.original?.id?.localeCompare(rowB.original?.id),
  },
  {
    accessorKey: 'created',
    header: ({ column }) => <SortableCell cellName="Created" column={column} />,
    cell: ({ row }) => <span className="text-[13px]">{fromNow(new Date(row.original.created * 1000).getTime())}</span>,
    sortingFn: (rowA, rowB) => (rowA.original.created < rowB.original.created ? 1 : -1),
  },
  {
    accessorKey: 'size',
    header: ({ column }) => <SortableCell cellName="Size" column={column} />,
    cell: ({ row }) => <span className="text-[13px]">{byteTransform(row.original.size, 2)}</span>,
    sortingFn: (rowA, rowB) => (rowA.original.size < rowB.original.size ? 1 : -1),
  },
  {
    id: 'actions',
    cell: ({ row }) => (
      <div className="text-center">
        <DropdownTableMenu image={row.original} />
      </div>
    ),
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
  const currentPlatform = useContextSelector(AppContext, (v) => v?.currentPlatform)!;
  const { data, isLoading, isSuccess } = useGETAllLocalImages(currentPlatform?.id);
  const setSelectedRows = useContextSelector(ImagesContext, (v) => v?.setSelectedRows)!;
  const setLocalImages = useContextSelector(ImagesContext, (v) => v?.setLocalImages)!;
  const localImages = useContextSelector(ImagesContext, (v) => v?.localImages)!;
  // Update local images when data is fetched
  useEffect(() => {
    if (isSuccess && data?.data.images) {
      setLocalImages(data.data.images);
    }
    return () => {
      setLocalImages([]);
      setSelectedRows([]);
    };
  }, [data, isSuccess, setLocalImages, setSelectedRows]);

  // Memoized selection change handler
  const handleSelectionChange = useCallback(
    (ids: string[]) => {
      setSelectedRows(localImages.filter((c) => ids.includes(c.id!)));
    },
    [localImages, setSelectedRows],
  );

  // Memoized row count
  const rowCount = useMemo(() => localImages?.length ?? 0, [localImages]);

  return (
    <div className="flex flex-col gap-3">
      <DataTable columns={columns} data={localImages} isLoading={isLoading} onSelectionChange={handleSelectionChange} />
      <div className="text-muted-foreground text-xs font-normal">
        {rowCount > 0 && (
          <span>
            Showing {rowCount} of {rowCount} image(s)
          </span>
        )}
      </div>
      <DeleteLocalImageDialog />
    </div>
  );
}

const ImageStatusTooltip = memo(({ inUse }: { inUse: boolean }) => {
  const getStatusClass = () => (inUse ? 'bg-green-500' : 'bg-gray-500');

  return (
    <TooltipProvider delayDuration={200}>
      <Tooltip>
        <TooltipTrigger asChild>
          <div className={`${getStatusClass()} mr-2 h-2 w-2 rounded-full`} />
        </TooltipTrigger>
        <TooltipContent>
          <span>{inUse ? 'In use' : 'Unused'}</span>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
});

ImageStatusTooltip.displayName = 'ImageStatusTooltip';
