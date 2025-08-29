import { DataTable } from '@/components/ui/data-table';
import { ImageView } from '@/api/_generated';
import SortableCell from '@/components/ui/SortableCell';
import { ColumnDef } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { useGETAllLocalImages } from './hooks/useGETAllLocalImages';
import DropdownTableMenu from './DropdownTableMenu';
import { useImagesContext } from './ImagesContext';
import { truncate } from '@/lib/truncate';
import { Play } from 'lucide-react';
import { useEffect, useCallback, useMemo } from 'react';
import { fromNow } from '@/lib/dayjs.helper';
import { byteTransform } from '@/lib/bytes.helper';
import { DeleteLocalImageDialog } from './dialogs/DeleteLocalImageDialog';
import { useAppContext } from '@/AppContext';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { Button } from '@/components/ui/button';
import { RunImageDialog } from './dialogs/RunImageDialog';
import { ImageSateIndicator } from './ImageStateIndicator';
import { useNavigate, useParams } from 'react-router';
import { CopyTextToClipboard } from '@/components/ui/CopyTextToClipboard';
import { formatImageId } from '@/lib/utils';

const columns = (): ColumnDef<ImageView>[] => [
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
    cell: ({ row }) => <ImageNameRow image={row.original} />,
    sortingFn: (rowA: any, rowB: any): number => rowA.original?.name?.localeCompare(rowB.original?.name),
  },
  {
    accessorKey: 'tag',
    header: ({ column }) => <SortableCell cellName="Tag" column={column} />,
    cell: ({ row }) => <div className="text-xs">{truncate(row.original.tag ?? '', 28)}</div>,
    sortingFn: (rowA: any, rowB: any): number => rowA.original?.tag?.localeCompare(rowB.original?.tag),
  },
  {
    accessorKey: 'id',
    header: ({ column }) => <SortableCell cellName="Image Id" column={column} />,
    cell: ({ row }) => (
      <CopyTextToClipboard
        textToCopy={row.original.id}
        transform={formatImageId}
        groupClassName="rowid"
        textClassName="text-xs"
      />
    ),
    sortingFn: (rowA: any, rowB: any): number => rowA.original?.id?.localeCompare(rowB.original?.id),
  },
  {
    accessorKey: 'created',
    header: ({ column }) => <SortableCell cellName="Created" column={column} />,
    cell: ({ row }) => (
      <span className="text-xs">{fromNow(new Date((row.original.created as number) * 1000).getTime())}</span>
    ),
    sortingFn: (rowA, rowB) => (rowA.original.created < rowB.original.created ? 1 : -1),
  },
  {
    accessorKey: 'size',
    header: ({ column }) => <SortableCell cellName="Size" column={column} />,
    cell: ({ row }) => <span className="text-xs">{byteTransform(row.original.size, 2)}</span>,
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

const ImageNameRow = ({ image }: { image: ImageView }) => {
  const { platformId } = useParams<{ platformId: string }>();
  const navigate = useNavigate();
  function onNameClick() {
    navigate(`/platforms/${platformId}/images/${formatImageId(image.id)}/`);
  }

  return (
    <div className="flex items-center whitespace-nowrap">
      <div className="flex items-center">
        <ImageSateIndicator inUse={image.isInUse ?? false} />
      </div>
      <span
        className="cursor-pointer text-[13px] table-link"
        onClick={onNameClick}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            onNameClick();
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

export default function LocalImagesTable() {
  const { currentPlatform } = useAppContext();
  const { data, isLoading, isSuccess } = useGETAllLocalImages(currentPlatform?.id);
  const {
    setSelectedRows,
    setLocalImages,
    localImages,
    setDialogData,
    dialogData,
    requestDelete,
    deleteIsPending,
    setRunDialogData,
    runDialogData,
  } = useImagesContext();
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
      <DataTable
        columns={columns()}
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
      <RunImageDialog runDialogData={runDialogData} setRunDialogData={setRunDialogData} />
      <DeleteLocalImageDialog
        dialogData={dialogData}
        setDialogData={setDialogData}
        requestDelete={requestDelete}
        deleteIsPending={deleteIsPending}
      />
    </div>
  );
}
