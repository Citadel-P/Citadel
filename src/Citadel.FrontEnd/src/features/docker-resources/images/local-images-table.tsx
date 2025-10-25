import { DataTable } from '@/components/ui/data-table';
import { ImageView } from '@/api/generated/api.types';
import SortableCell from '@/components/ui/SortableCell';
import { ColumnDef } from '@tanstack/react-table';
import { Checkbox } from '@/components/ui/checkbox';
import { TableDropDown } from './table-dropdown';
import { truncate } from '@/lib/truncate';
import { Play } from 'lucide-react';
import { useMemo } from 'react';
import { fromNow } from '@/lib/dayjs.helper';
import { byteTransform } from '@/lib/bytes.helper';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { Button } from '@/components/ui/button';
import { useNavigate, useParams } from 'react-router';
import { CopyToClipboard } from '@/components/custom/copy-to-clipboard';
import { formatId } from '@/lib/utils';
import { RegistryDisplay } from '@/components/ui/RegistryDisplay';
import { StateIndicator } from '@/components/custom/state-indicator';
import { useSelectedResources } from '@/lib/atoms';

export const LocalImagesTable = ({ items, isLoading }: { items: ImageView[]; isLoading: boolean }) => {
  const rowCount = useMemo(() => items?.length ?? 0, [items]);
  const [_, setSelectedResources] = useSelectedResources<ImageView>('Image');

  return (
    <div className="flex flex-col gap-3">
      <DataTable
        columns={columns()}
        data={items ?? []}
        isLoading={isLoading}
        onSelectionChange={setSelectedResources}
      />
      <div className="text-muted-foreground text-xs p-2 font-normal">
        {rowCount > 0 && (
          <span>
            Showing {rowCount} of {rowCount} image(s)
          </span>
        )}
      </div>
    </div>
  );
};

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
    cell: ({ row }) => <div className="">{truncate(row.original.tag ?? '', 28)}</div>,
    sortingFn: (rowA: any, rowB: any): number => rowA.original?.tag?.localeCompare(rowB.original?.tag),
  },
  {
    accessorKey: 'registry',
    header: ({ column }) => <SortableCell cellName="Registry" column={column} />,
    cell: ({ row }) => <RegistryDisplay registry={row.original.registry ?? undefined} />,
    sortingFn: (rowA: any, rowB: any): number =>
      rowA.original.registry?.name?.localeCompare(rowB.original.registry?.name),
  },
  {
    accessorKey: 'id',
    header: ({ column }) => <SortableCell cellName="Image Id" column={column} />,
    cell: ({ row }) => (
      <CopyToClipboard
        textToCopy={row.original.dockerImageId}
        transform={formatId}
        groupClassName="rowid"
        textClassName=""
      />
    ),
    sortingFn: (rowA: any, rowB: any): number => rowA.original?.imageId?.localeCompare(rowB.original?.imageId),
  },
  {
    accessorKey: 'created',
    header: ({ column }) => <SortableCell cellName="Created" column={column} />,
    cell: ({ row }) => <span className="">{fromNow(new Date(row.original.createdAt).getTime())}</span>,
    sortingFn: (rowA, rowB) => (rowA.original.createdAt < rowB.original.createdAt ? 1 : -1),
  },
  {
    accessorKey: 'size',
    header: ({ column }) => <SortableCell cellName="Size" column={column} />,
    cell: ({ row }) => <span className="">{byteTransform(row.original.size, 2)}</span>,
    sortingFn: (rowA, rowB) => (rowA.original.size < rowB.original.size ? 1 : -1),
  },
  {
    id: 'actions',
    cell: ({ row }) => <RenderActions image={row.original} />,
  },
];

const RenderActions = ({ image }: { image: ImageView }) => {
  return (
    <div className="flex justify-center">
      <TooltipProvider delayDuration={200}>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon" className="size-8 rounded-full" onClick={() => null}>
              <Play className="text-blue-500" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            <span>Create</span>
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>
      <TableDropDown image={image} />
    </div>
  );
};

const ImageNameRow = ({ image }: { image: ImageView }) => {
  const { platformId } = useParams<{ platformId: string }>();
  const navigate = useNavigate();
  function onNameClick() {
    navigate(`/platforms/${platformId}/images/${formatId(image.dockerImageId)}/`);
  }

  return (
    <div className="flex items-center whitespace-nowrap">
      <div className="flex items-center">
        <StateIndicator value={image.isInUse} />
      </div>
      <span
        className="cursor-pointer table-link"
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
