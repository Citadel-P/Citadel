import { DataTable } from '@/components/ui/data-table';
import { ImageView } from '@/api/_generated';
import SortableCell from '@/components/ui/SortableCell';
import { ColumnDef } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { useGETAllLocalImages } from './hooks/useGETAllLocalImages';
import DropdownTableMenu from './DropdownTableMenu';
import { useImagesContext } from './ImagesContext';
import { truncate } from '@/lib/truncate';
import { useCopyToClipboard } from '@/hooks/useCopyToClipboard';
import { CheckCheck, Play, Clipboard } from 'lucide-react';
import { useEffect, useCallback, useMemo, memo } from 'react';
import { fromNow } from '@/lib/dayjs.helper';
import { byteTransform } from '@/lib/bytes.helper';
import { DeleteLocalImageDialog } from './dialogs/DeleteLocalImageDialog';
import { useAppContext } from '@/AppContext';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { ImageInspectSheet } from './ImageInspectSheet';
import { Button } from '@/components/ui/button';
import { RunImageDialog } from './dialogs/RunImageDialog';

const columns = (handleShowSheet: (network: ImageView) => void): ColumnDef<ImageView>[] => [
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
    cell: ({ row }) => <ImageNameRow image={row.original} onShowSheet={handleShowSheet} />,
    sortingFn: (rowA: any, rowB: any): number => rowA.original?.name?.localeCompare(rowB.original?.name),
  },
  {
    accessorKey: 'tag',
    header: ({ column }) => <SortableCell cellName="Tag" column={column} />,
    cell: ({ row }) => <div>{truncate(row.original.tag ?? '', 28)}</div>,
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
    cell: ({ row }) => (
      <span className="text-[13px]">{fromNow(new Date((row.original.created as number) * 1000).getTime())}</span>
    ),
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
    cell: ({ row }) => <RenderActions image={row.original} />,
  },
];

const RenderActions = ({ image }: { image: ImageView }) => {
  const { setRunDialogData } = useImagesContext();

  return (
    <div className="flex justify-center">
      <TooltipProvider delayDuration={200}>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              className="size-8 rounded-full"
              onClick={() => setRunDialogData({ open: true, currentSelection: [image] })}>
              <Play className="text-blue-500" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            <span>Create</span>
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>

      <div className="text-center">
        <DropdownTableMenu image={image} />
      </div>
    </div>
  );
};

const ImageNameRow = ({ image, onShowSheet }: { image: ImageView; onShowSheet: (image: ImageView) => void }) => {
  return (
    <div className="flex items-center whitespace-nowrap">
      <div className="flex items-center">
        <ImageStatusTooltip inUse={image.isInUse ?? false} />
      </div>
      <span
        className="cursor-pointer hover:underline"
        onClick={() => onShowSheet(image)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            onShowSheet(image);
          }
        }}
        tabIndex={0}
        role="button"
        aria-label="Show image details">
        {truncate(image.name ?? '', 32, 'right')}
      </span>
    </div>
  );
};

const ImageIdRow = ({ image }: { image: ImageView }) => {
  const [copyCmd, setCopyCmd] = useCopyToClipboard(5000);

  return (
    <div className="flex gap-0.5 items-center">
      <div>{truncate(image.id?.split(':').at(1) ?? '', 12, 'right', true)}</div>
      <button
        className="rounded-full invisible group-hover/trow:visible ml-1 px-1.5 py-1.5 bg-foreground/5 hover:bg-foreground/10 text-sm font-semibold"
        onClick={() => setCopyCmd(image.id ?? '')}>
        {copyCmd ? <CheckCheck className="w-3 h-3 text-green-500" /> : <Clipboard className="w-3 h-3 " />}
      </button>
    </div>
  );
};

export default function LocalImagesTable() {
  const { currentPlatform } = useAppContext();
  const { data, isLoading, isSuccess } = useGETAllLocalImages(currentPlatform?.id);
  const { setSelectedRows, setLocalImages, localImages, setCurrentImage, setSheetOpen } = useImagesContext();
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

  const handleShowSheet = (image: ImageView) => {
    setCurrentImage(image);
    setSheetOpen(true);
  };
  // Memoized row count
  const rowCount = useMemo(() => localImages?.length ?? 0, [localImages]);

  return (
    <div className="flex flex-col gap-3">
      <DataTable
        columns={columns(handleShowSheet)}
        data={localImages}
        isLoading={isLoading}
        onSelectionChange={handleSelectionChange}
      />
      <div className="text-muted-foreground text-xs font-normal">
        {rowCount > 0 && (
          <span>
            Showing {rowCount} of {rowCount} image(s)
          </span>
        )}
      </div>
      <RunImageDialog />
      <DeleteLocalImageDialog />
      <ImageInspectSheet />
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
